# ETH2 State Transition Function — zkVM 検証結果

Ethereum Consensus Layer（Beacon Chain）の state transition function を Lean 4 で仕様に忠実に実装し、RISC Zero zkVM 上で正しく動作するかを検証した。比較対象として同等ロジックの Pure Rust 実装も用意し、3構成で検証を行った。

参照仕様: [eth2book](https://eth2book.info/latest/part3/transition/) / [ethereum/consensus-specs](https://github.com/ethereum/consensus-specs)

## 検証構成

| 構成 | 言語 | Init 初期化 | 説明 |
|------|------|------------|------|
| **Lean (no-init)** | Lean 4 | なし | `risc0_main_eth2()` を直接呼出。Init 初期化をスキップ |
| **Lean (init)** | Lean 4 | あり | Init_Data ワークアラウンド適用後に STF 呼出 |
| **Rust** | Rust | — | 同等ロジックの Pure Rust 実装（ベースライン） |

テスト入力: slot 100 → 101 の 1-slot advance（エポック境界なし）。暗号プリミティブは全てスタブ。

---

## ベンチマーク結果

`--suite eth2 --mode execute`（execute モード、`RISC0_DEV_MODE=1`）。

### サイクル数・セグメント数

| Guest | N=10 バリデータ | N=100 バリデータ |
|-------|----------------|-----------------|
| Lean (no-init) | **CRASHED** | **CRASHED** |
| Lean (init) | 26,148,291 cycles / 29 seg | 35,281,299 cycles / 38 seg |
| Rust | 12,491,509 cycles / 13 seg | 14,446,747 cycles / 15 seg |
| **Lean/Rust 比率** | **2.1x** / 2.2x | **2.4x** / 2.5x |

### ELF サイズ・出力サイズ

| | Lean (no-init) | Lean (init) | Rust |
|--|--:|--:|--:|
| ELF サイズ | 2.6 MB | 6.6 MB | 373 KB |
| ELF サイズ比率 (vs Rust) | 6.9x | 17.7x | 1.0x |
| 出力 (N=10) | — | 78,746 B | 78,746 B |
| 出力 (N=100) | — | 91,976 B | 91,976 B |

**Lean (init) と Rust の出力はバイト単位で完全一致**しており、意味論的に同等な STF が動作していることを確認した。

---

## Init スキップ実験

### 核心的な問い

> Init ライブラリ初期化をスキップしても、意味論的に正しい STF が zkVM 上で再現できるか？

### 結論: Init スキップは不可能

#### Lean (no-init) — CRASHED

```
Invalid trap address: 0x00000000, cause: LoadAccessFault(0x00000008)
```

Lean コンパイラが生成する C IR では、`default` 値・空配列 `#[]`・リテラル `ByteArray.mk #[0xFF]` 等が **closed term** として BSS セグメントに配置される。これらは `initialize_Guest()` 実行時に初期化されるが、Init なしでは全て **NULL のまま**。`risc0_main_eth2()` がこれらにアクセスした時点で NULL dereference → LoadAccessFault が発生する。

**結論**: `Array`、`String`、`ByteArray`、`default` 等の Lean 標準機能を使う限り、**Init は必須**。`UInt32` 等のアンボックス型のみを使う sum 関数とは本質的に異なる。

### デバッグ過程（バイナリサーチによる原因特定）

クラッシュの根本原因を特定するため、C ラッパーを段階的に修正して二分探索を行った。

| テスト | 内容 | 結果 |
|--------|------|------|
| 1 | Init のみ、静的バッファ返却 | PASS (15.8M cycles) |
| 2 | Init + `risc0_main(10)` 呼出 | PASS |
| 3 | Init + `risc0_main_eth2(empty)` + 結果アクセス | CRASH |
| 3b | Init + `risc0_main_eth2(empty)` + 結果無視 | PASS |
| 3c | Init + `risc0_main_eth2(empty)` + `lean_sarray_size` | CRASH |
| 5 | `risc0_main_eth2` の戻り値を raw ポインタとして出力 | **戻り値 = NULL** |
| 8 | closed term アドレス (0x44d1878) のメモリ読取 | **Guest 初期化後も NULL** |
| 9 | どの初期化ステップが失敗するか診断 | **`initialize_Init` が失敗** |
| 12 | Init のサブモジュールを個別テスト | **`initialize_Init_Data` が失敗** |
| 13 | Data を先行呼出 → Init → Guest | **全て成功** |
| 14 | ワークアラウンド + `risc0_main_eth2(empty)` | **`#[0xFF]` 正常返却** |
| 15 | ワークアラウンド + 実データ | **STF 実行成功** |

### initialize_Init_Data の失敗原因

`initialize_Init()` → `initialize_Init_Data()` の呼出が zkVM 上で失敗する。

**原因チェーン**:

1. `Init_Data` の初期化パスが libc のファイル操作を呼び出す
2. zkVM の `shims.c` では全ファイル操作が `-1` を返す
3. しかし `strerror(0)` は "success" を返す（errno が 0 のまま）
4. `Init_Data` はエラーコード 0 を「失敗」と判定
5. エラーメッセージ: `"success (error code: 0)"`

### ワークアラウンド

`initialize_Init_Data()` は失敗するが、内部的に `_G_initialized = true` フラグをセットする。その後 `initialize_Init()` を呼ぶと、Init_Data は「既に初期化済み」としてスキップされ、初期化全体が成功する。

```c
// methods/guest-eth2-init/risc0_lean.c
lean_initialize_runtime_module(lean_io_mk_world());  // (1) ランタイム初期化
initialize_Init_Data(1, lean_io_mk_world());         // (2) 失敗するが _G_initialized=true をセット
initialize_Init(1, lean_io_mk_world());              // (3) Data をスキップして成功
initialize_Guest(1, lean_io_mk_world());             // (4) 全 closed term を初期化
```

---

## オーバーヘッド分析

### サイクル数オーバーヘッド (2.1x〜2.4x)

Lean/Rust 比率が N=10 で 2.1x、N=100 で 2.4x と増加する。要因:

1. **Init 固定コスト（~15M cycles）**: 392 モジュールの初期化。N=10 では全体の約 57% を占める
2. **永続データ構造のコスト**: Lean の `Array.set!` は参照カウントが 1 でない場合コピーが発生する。バリデータ数が増えるとこの影響が増大（N=10 → N=100 で比率 2.1x → 2.4x に上昇）
3. **参照カウント**: `{ state with ... }` による構造体更新時の RC increment/decrement 操作

### ELF サイズオーバーヘッド (17.7x)

| 構成要素 | サイズ寄与 |
|----------|-----------|
| Init ライブラリ（392 モジュール） | ~4.0 MB |
| Lean ランタイム (`libLean.a`) | ~1.0 MB |
| Guest コード (`libGuest.a`) | ~2.5 MB |
| libc + libstdc++ | ~0.1 MB |
| **Lean (init) 合計** | **6.6 MB** |
| **Rust 合計** | **373 KB** |

---

## Lean STF 実装

### モジュール構成

```
guest/Guest/Eth2/
  Types.lean            -- Slot, Epoch, Gwei, Root 等の型エイリアス
  Constants.lean        -- SLOTS_PER_EPOCH, MAX_EFFECTIVE_BALANCE 等
  Crypto.lean           -- hashTreeRoot, blsVerify（スタブ）
  Containers.lean       -- BeaconState, BeaconBlock, Validator 等の構造体
  Helpers.lean          -- getCurrentEpoch, getBaseReward, isActiveValidator 等
  Serialize.lean        -- BeaconState → ByteArray シリアライゼーション
  Decode.lean           -- ByteArray → 型 デシリアライゼーション
  Transition/
    StateTransition.lean    -- state_transition, process_slots, process_slot
    Epoch.lean              -- process_epoch（12 sub-functions）
    Block.lean              -- process_block ディスパッチャ
    Block/
      Header.lean           -- process_block_header
      Randao.lean           -- process_randao
      Eth1Data.lean         -- process_eth1_data
      Operations.lean       -- process_operations ディスパッチャ
      SyncAggregate.lean    -- process_sync_aggregate
```

### 仕様との対応

| 項目 | 実装状況 |
|------|----------|
| 型定義 | Altair/Bellatrix 仕様に準拠（`Slot = UInt64`, `Gwei = UInt64`, `Root = ByteArray` 等） |
| Epoch 処理 | 12 sub-functions を仕様順に実行 |
| Block 処理 | header → randao → eth1_data → operations → sync_aggregate |
| 暗号プリミティブ | 全てスタブ（`hashTreeRoot → 固定値`, `blsVerify → true`） |
| proposer 選出 | `slot % active_validator_count`（RANDAO シャッフルなし） |
| Withdrawals | スタブ（Capella 以降の機能） |
| ExecutionPayload | スタブ（ヘッダ保存のみ） |

### エントリポイント

```lean
-- guest/Guest.lean
@[export risc0_main_eth2]
def risc0_main_eth2 (input : @& ByteArray) : ByteArray :=
  -- ByteArray をデコード → state_transition → 結果をシリアライズ
  -- 成功時: シリアライズされた post-state BeaconState
  -- 失敗時: 0xFD + UTF-8 エラーメッセージ
  -- デコード失敗時: 0xFF (BeaconState) / 0xFE (SignedBeaconBlock)
```

### FFI パイプライン

```
Rust guest (main.rs)
  → extern "C" lean_eth2_init_entry(input, len, &output, &output_len)
    → risc0_lean.c: Init_Data workaround → initialize_Init → initialize_Guest
    → risc0_main_eth2(lean_input)  // Lean FFI
      → Guest.lean: decode → stateTransition → serialize
    → return ByteArray as raw bytes
  → env::commit(&output)
```

---

## Guest crate 構成

### methods/guest-eth2-init/（動作する構成）

```
methods/guest-eth2-init/
  Cargo.toml        -- risc0-zkvm + cc dependency
  build.rs          -- links libGuest.a, libLean.a, libInit.a, libc, libstdc++
  src/main.rs       -- Rust guest entry: read input → FFI → commit output
  risc0_lean.c      -- C wrapper with Init_Data workaround
  shims.c           -- syscall stubs for zkVM (64MB heap, no I/O)
  lib/libGuest.a    -- Lean compiled to RISC-V (build artifact, gitignored)
```

### methods/guest-eth2-noinit/（クラッシュする構成）

同構造だが `risc0_lean.c` が `initialize_Guest()` を呼ばない。Init スキップ実験の対照群。

### methods/guest-rust-eth2/（ベースライン）

```
methods/guest-rust-eth2/
  Cargo.toml        -- risc0-zkvm only (no FFI)
  src/main.rs       -- Rust guest: read input → STF → commit output
  src/types.rs      -- Rust 型定義
  src/transition.rs -- Rust STF 実装
```

---

## 結論

| 問い | 回答 |
|------|------|
| Init なしで ETH2 STF は動くか？ | **動かない**。closed term が未初期化で NULL dereference が発生 |
| Init ありで動くか？ | **動く**。Init_Data ワークアラウンドが必要 |
| Lean と Rust で同一出力か？ | **バイト単位で完全一致** |
| サイクル数オーバーヘッドは？ | **2.1x〜2.4x**（バリデータ数に依存） |
| ELF サイズオーバーヘッドは？ | **17.7x**（主に Init ライブラリの静的リンク） |

### 今後の課題

- [ ] 暗号プリミティブの実装（`hash_tree_root`, BLS 検証）
- [ ] RANDAO ベースの proposer 選出
- [ ] エポック境界を跨ぐテストケース
- [ ] より大規模なバリデータセット（1,000+）での計測
- [ ] Init 固定コスト削減の調査（不要モジュールの除外等）

## 再現手順

```bash
export RISC0_TOOLCHAIN_PATH="$HOME/.risc0/toolchains/v2024.1.5-cpp-aarch64-apple-darwin/riscv32im-osx-arm64"
export LEAN_RISC0_PATH="$HOME/.lean-risc0"

# Full build
just clean && just build

# Run 3-way benchmark
just bench-eth2-execute

# Or manually with custom parameters
RISC0_DEV_MODE=1 cargo run --release --bin benchmark -- \
  --suite eth2 --mode execute --inputs 10,100 --guest all
```
