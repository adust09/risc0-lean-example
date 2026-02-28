# zkVM スナップショット機構調査 — Init 初期化コスト削減

## 背景

Lean guest が zkVM 上で実行されるたびに Init 標準ライブラリの初期化（~15M cycles）が発生する。ETH2 STF ベンチマーク（N=10 validators）では全体 26.1M cycles のうち約 57% を占め、最大のボトルネックとなっている。

この初期化は入力に依存しない（deterministic）ため、zkVM レベルのスナップショット機構で削減可能かを調査した。

| 指標 | ETH2 STF (N=10) | ETH2 STF (N=100) | sum (Nat) |
|------|-----------------|-------------------|-----------|
| Init 固定コスト | ~15M cycles | ~15M cycles | ~4.1M cycles |
| 全体に占める割合 | **57%** | **43%** | **99%** |
| 全体サイクル数 | 26,148,291 | 35,281,299 | 4,119,771 |

---

## 調査対象: risc0-zkvm v3.0.5 の関連 API

### MemoryImage（risc0-binfmt v3.0.3）

ゲストプログラムのメモリ状態全体（メモリページ・プログラムカウンタ等）を表す構造体。`serde::Serialize` / `serde::Deserialize` を実装しており、シリアライズ可能。

```rust
// risc0-binfmt/src/image.rs
pub struct MemoryImage { /* ... */ }

impl MemoryImage {
    pub fn new(program: Program, pc: u32) -> Result<Self>
}
```

### ExecutorImpl（risc0-zkvm v3.0.5）

ゲスト実行エンジン。任意の `MemoryImage` を受け取るコンストラクタが公開されている。

```rust
// risc0-zkvm/src/host/server/exec/executor.rs
pub struct ExecutorImpl<'a> {
    image: MemoryImage,  // pub(crate) — 外部からアクセス不可
    // ...
}

impl<'a> ExecutorImpl<'a> {
    pub fn new(env: ExecutorEnv<'a>, image: MemoryImage) -> Result<Self>
    pub fn from_elf(env: ExecutorEnv<'a>, elf: &[u8]) -> Result<Self>
    pub fn run(&mut self) -> Result<Session>
}
```

`run()` の内部で `self.image = result.post_image.clone()` が実行され、pause 後に再度 `run()` を呼ぶと pause 時点の状態から自動的に再開する。

### pause / resume（ゲスト側 API）

```rust
// risc0_zkvm::guest::env
pub fn pause(exit_code: u8)
```

ゲストが `pause()` を呼ぶと、ホスト側の `run()` が `Session { exit_code: ExitCode::Paused(n), .. }` を返す。同一 `ExecutorImpl` インスタンスで再度 `run()` を呼べば、pause 地点から実行を再開する。

### ExitCode

```rust
// risc0-binfmt/src/exit_code.rs
pub enum ExitCode {
    Halted(u32),     // normal termination
    Paused(u32),     // guest-initiated pause, resumable
    SystemSplit,     // automatic segment boundary (continuations)
    SessionLimit,    // execution limit reached
}
```

### CompositeReceipt

複数の `SegmentReceipt` を合成して単一の検証可能な receipt を構成する。セグメント N の post_state がセグメント N+1 の pre_state と一致する必要がある。

---

## 3つの削減アプローチ

### Approach A: pause/resume による証明分離（推奨 — 公開 API のみで実現可能）

#### 概要

ゲスト実行を 2 フェーズに分割し、Init 初期化後に `pause()` を挿入する。

```
Phase 1: Init initialization  → env::pause(0)     [Segment 0: ~15M cycles]
Phase 2: input read → STF → commit                [Segment 1: ~11M cycles]
```

Init は入力に依存しないため、ゲストが `env::read()` を pause 後まで遅延させれば、Segment 0 の proof は全入力で同一になる。Init の proof を 1 回生成して再利用し、入力ごとに Segment 1 の proof のみを生成する。

#### ゲスト側の変更イメージ

```rust
// methods/guest-eth2-init/src/main.rs
fn main() {
    // Phase 1: Init only (input-independent)
    unsafe { lean_eth2_init_only(); }
    env::pause(0);  // Segment 0 ends here

    // Phase 2: Business logic (Segment 1)
    let input: Vec<u8> = env::read();
    let mut output_ptr: *mut u8 = std::ptr::null_mut();
    let mut output_len: usize = 0;
    unsafe {
        lean_eth2_process(
            input.as_ptr(), input.len(),
            &mut output_ptr, &mut output_len,
        );
    }
    let result = unsafe {
        std::slice::from_raw_parts(output_ptr, output_len)
    };
    env::commit_slice(result);
}
```

#### ホスト側の実行コード

```rust
use risc0_zkvm::ExecutorImpl;

let env = build_eth2_env(test_input);
let mut exec = ExecutorImpl::from_elf(env, GUEST_ETH2_INIT_ELF)?;

// Segment 0: Init (~15M cycles)
let init_session = exec.run()?;
assert_eq!(init_session.exit_code, ExitCode::Paused(0));

// Segment 1: Business logic (~11M cycles)
let biz_session = exec.run()?;
assert_eq!(biz_session.exit_code, ExitCode::Halted(0));
```

#### 実現可能性と制約

| 項目 | 評価 |
|------|------|
| API 互換性 | `pause()`, `ExecutorImpl::run()`, `ExitCode` 全て公開 API |
| 実装難度 | 低 — ゲスト/ホスト双方に小さな変更のみ |
| execute モードへの影響 | **なし** — 全サイクルは依然として実行される |
| prove モードへの影響 | **Init proof を 1 回だけ生成して再利用可能** |
| 期待される効果 | 2 回目以降の prove で **~57% の証明コスト削減**（N=10 の場合） |
| stdin の扱い | `ExecutorEnv` 構築時に提供。ゲストが Phase 2 で `env::read()` を呼ぶまでバッファに保持される |

#### 証明合成の仕組み

```
CompositeReceipt {
    segments: [
        SegmentReceipt { pre: ELF_ImageID, post: PostInit_StateHash },  // reusable
        SegmentReceipt { pre: PostInit_StateHash, post: Final_StateHash },  // per-input
    ]
}
```

Init の `SegmentReceipt` は post_state が常に同一（初期化結果は決定的）なため、異なる入力の Segment 1 と合成可能。

---

### Approach B: MemoryImage シリアライズ（risc0-zkvm のフォークが必要）

#### 概要

1. Init 実行を 1 回行い、pause 後の `MemoryImage` を抽出
2. `MemoryImage` をディスクにシリアライズ（serde 対応済み）
3. 以降の実行では、保存した `MemoryImage` を `ExecutorImpl::new(env, saved_image)` に渡して Init をスキップ

#### 期待効果

execute / prove の **両方** で ~15M cycles を完全に排除。

#### 致命的な制約: `pub(crate)` フィールド

```rust
// ExecutorImpl 内部
image: MemoryImage,  // pub(crate) — crate 外から読み取り不可
```

`run()` 完了後の `MemoryImage`（= pause 後のメモリ状態）は `ExecutorImpl` の `pub(crate)` フィールドに格納される。外部クレートからこの値を取得する公開 API は存在しない。

**実現に必要な作業:**

- risc0-zkvm をフォークし、`pub fn image(&self) -> &MemoryImage` アクセサを追加
- または upstream に PR を提出して公開 API 化を提案

#### 証明上の注意点

保存した `MemoryImage` から開始すると、proof の起点（Image ID）が元の ELF の Image ID と異なる。検証者はスナップショットの Image ID を信頼する必要がある。

---

### Approach C: バッチ実行（最も単純）

#### 概要

複数の入力を単一のゲスト実行で処理し、Init コストを分散する。

```
Init → process(input_1) → process(input_2) → ... → commit_all
```

#### 評価

| 項目 | 評価 |
|------|------|
| API 変更 | 不要 — 即座に実装可能 |
| Init コスト | N 個の入力で 1/N に分散 |
| 制約 | 入力ごとの個別 proof を生成できない。全入力が単一 receipt にバンドルされる |
| 適用場面 | バッチ処理が許容されるユースケース（例: 複数スロットの一括検証） |

---

## アプローチ比較

| | Approach A | Approach B | Approach C |
|--|-----------|-----------|-----------|
| **要約** | pause/resume + 証明分離 | MemoryImage 保存・復元 | バッチ実行 |
| **公開 API のみ** | Yes | **No** (フォーク必要) | Yes |
| **execute 高速化** | No | **Yes** (~15M cycles 削減) | Yes (per-input) |
| **prove 高速化** | **Yes** (Init proof 再利用) | **Yes** (Init 完全スキップ) | Yes (per-input) |
| **個別 proof** | Yes | Yes | **No** |
| **実装難度** | 低 | 中（フォーク管理） | 低 |
| **推奨度** | **PoC 最優先** | 中長期検討 | 限定的 |

---

## 現時点の API 制約まとめ

### 1. `ExecutorImpl.image` が `pub(crate)`

`run()` 完了後の `MemoryImage` を外部から読み取れない。Approach B の最大の障壁。risc0-zkvm の upstream に MemoryImage アクセサの公開を提案する価値がある。

### 2. ExecutorEnv の stdin は構築時に固定

`ExecutorEnvBuilder::write()` / `write_slice()` で設定した stdin バッファは構築後に変更できない。

- **Approach A**: 同一セッション内の pause/resume では元のバッファが保持されるため問題なし。ゲストが Phase 2 で `env::read()` を呼べばバッファから読み取れる。
- **Approach B**: 新しい `ExecutorImpl` に新しい `ExecutorEnv` を渡せるため、入力の差し替えは可能。

### 3. ベンチマークコードの抽象化

ベンチマークで使用する `default_executor()` は trait object を返し、`ExecutorImpl` の `run()` メソッドを直接呼べない。pause/resume を利用するには `ExecutorImpl` を直接構築する必要がある。

---

## 参考: Init 初期化の内部動作

Init の初期化シーケンス（C ラッパー `risc0_lean.c` 内）:

```c
// Step 1: Lean runtime initialization
lean_initialize_runtime_module(lean_io_mk_world());

// Step 2: Init_Data workaround
// initialize_Init_Data() は内部で libc ファイル操作を呼ぶが、
// zkVM の shims.c が -1 を返すため失敗する。
// ただし _G_initialized フラグはセットされる。
initialize_Init_Data(1, lean_io_mk_world());

// Step 3: Init initialization (Step 2 の flag により Data を skip)
initialize_Init(1, lean_io_mk_world());

// Step 4: Guest code initialization
initialize_Guest(1, lean_io_mk_world());
```

Init ライブラリは 392 モジュールを初期化し、これが ~15M cycles の固定コストを生む。この処理は完全に決定的（deterministic）であり、入力やランタイム状態に依存しない。

## 次のステップ

1. **Approach A の PoC 実装**: ゲストコードに `pause()` を挿入し、ホスト側で `ExecutorImpl` を直接使用して段階的実行を検証する
2. **prove モードでの計測**: Init proof と Business proof を分離生成し、実際の時間削減を計測する
3. **upstream 提案の検討**: Approach B 実現のため、risc0-zkvm に `MemoryImage` アクセサ追加の issue/PR を検討する
