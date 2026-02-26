# Lean ゲスト オーバーヘッド分析

RISC Zero zkVM 上で Lean 4 コードを実行した際のパフォーマンスオーバーヘッドを、
同等の純 Rust 実装と比較して分析する。

## ベンチマーク概要

**テスト関数:** `sum(n) = if n == 0 then 0 else (n + sum(n-1)) &&& 0xFFFF`

両ゲストは同一のアルゴリズムを計算する。Lean ゲストは Lean → C → RISC-V の
フルパイプラインを経由し、Rust ゲストは直接実装である。

### 実行モード結果（証明なし）

| ゲスト | N | ユーザーサイクル | セグメント | 実行時間 |
|--------|------:|-----------:|---------:|---------:|
| Lean   |    10 |  4,119,771 |        5 |   12.3s  |
| Rust   |    10 |      3,736 |        1 |    41ms  |
| **倍率** | | **1,102.7x** | **5.0x** | **~300x** |
| | | | | |
| Lean   |   100 |  4,124,252 |        5 |   14.6s  |
| Rust   |   100 |      5,176 |        1 |    39ms  |
| **倍率** | | **796.8x** | **5.0x** | **~375x** |
| | | | | |
| Lean   | 1,000 |  4,161,410 |        6 |   10.4s  |
| Rust   | 1,000 |     19,576 |        1 |    74ms  |
| **倍率** | | **212.6x** | **6.0x** | **~140x** |

### ELF バイナリサイズ

| | Lean | Rust | 倍率 |
|--|-----:|-----:|-----:|
| ELF サイズ | 5,307,860 bytes (5.1 MB) | 276,472 bytes (270 KB) | **19.2x** |

## オーバーヘッドの4層構造

### 第1層: Init ライブラリ初期化（サイクルの ~99%）

最大のオーバーヘッド源。入力サイズに関係なくユーザーサイクルがほぼ一定であることが
明確な証拠となる。

```
N=10:    4,119,771 cycles
N=100:   4,124,252 cycles  （N=10 との差分: +4,481 = 実際の計算分）
N=1000:  4,161,410 cycles  （N=10 との差分: +41,639 = 実際の計算分）
```

基底の ~4,119,000 サイクルは全て `initialize_Init()` と
`lean_initialize_runtime_module()` に費やされている。

**呼び出しチェーン** (`methods/guest/risc0_lean.c` → `guest_build/risc0_ir/Guest.c`):

```c
// risc0_lean.c:lean_risc0_main()
lean_initialize_runtime_module();                    // Lean ランタイム初期化
lean_object* res = initialize_Guest(1, lean_io_mk_world());

// Guest.c:initialize_Guest()
res = initialize_Init(builtin, lean_io_mk_world()); // ← 全サイクルの ~99%
res = initialize_Guest_Basic(builtin, lean_io_mk_world());
```

**メカニズム:** Lean の `import` は推移的。`Guest.Basic` が `Init` をインポートする
だけで、Init ライブラリの 392 モジュール全てが `initialize_Init()` で再帰的に
初期化される。各モジュールは静的変数（文字列リテラル、ルックアップテーブル、
型情報）をヒープに確保し、`lean_mark_persistent()` で永続化する。

**ライブラリ内訳:**

| ライブラリ | サイズ | オブジェクトファイル数 | 役割 |
|-----------|-------:|-------------------:|------|
| `libInit.a` | 23 MB | 394 | Init ライブラリ（392 個の `initialize_*` 関数） |
| `libLean.a` | 944 KB | 44 | ランタイム（GC、参照カウント、メモリ管理） |
| `libGuest.a` | 9.6 KB | 2 | ユーザーコード（`Guest.c` + `Guest/Basic.c`） |

ユーザーコードはわずか 9.6 KB だが、ビジネスロジックの実行前に 23 MB の
初期化コードを全て実行する必要がある。

### 第2層: Nat ヒープ割り当て + 参照カウント

Lean の `Nat` 型はヒープ確保されるボックス化オブジェクトである。
全ての算術演算が新しい `lean_object` を確保し、参照カウントを必要とする。

**Lean C IR** (`guest_build/risc0_ir/Guest/Basic.c`):

```c
LEAN_EXPORT lean_object* l_sum(lean_object* x_1) {
    x_2 = lean_unsigned_to_nat(0u);     // 0 を Nat オブジェクトに変換
    x_3 = lean_nat_dec_eq(x_1, x_2);    // 比較
    if (x_3 == 0) {
        x_4 = lean_unsigned_to_nat(1u);  // 1 を Nat オブジェクトに変換
        x_5 = lean_nat_sub(x_1, x_4);   // ヒープ確保: n - 1
        x_6 = l_sum(x_5);               // 再帰呼び出し
        lean_dec(x_5);                   // 参照カウント減少
        x_7 = lean_nat_add(x_1, x_6);   // ヒープ確保: n + sum(n-1)
        lean_dec(x_6);                   // 参照カウント減少
        x_8 = lean_unsigned_to_nat(65535u);
        x_9 = lean_nat_land(x_7, x_8);  // ヒープ確保: result & 0xFFFF
        lean_dec(x_7);                   // 参照カウント減少
        return x_9;
    }
    return x_2;
}
```

**同等の Rust 実装** (`methods/guest-rust/src/main.rs`):

```rust
fn sum(n: u32) -> u32 {
    if n == 0 { 0 } else { (n + sum(n - 1)) & 0xFFFF }
}
```

**1回の再帰あたりのオーバーヘッド:**

| | Lean | Rust |
|--|------|------|
| ヒープ割り当て | 3回（`lean_nat_sub`, `lean_nat_add`, `lean_nat_land`） | 0回 |
| 参照カウント操作 | 3回（`lean_dec`） | 0回 |
| 型変換 | 3回（`lean_unsigned_to_nat`） | 0回 |
| サイクル数 | ~50 cycles/再帰 | ~16 cycles/再帰 |

**算出根拠:** (4,124,252 - 4,119,771) / 90 反復 ≈ ~50 cycles/再帰（Lean）。
(5,176 - 3,736) / 90 ≈ ~16 cycles/再帰（Rust）。

### 第3層: データマーシャリング（FFI 境界）

Rust ゲストから Lean に `u32` を直接渡すことができない。
データは各方向で6段階の変換パイプラインを通過する。

```
入力: u32 → String → bytes → [C FFI] → ByteArray → String → Nat
                                                               ↓
                                                            sum(n)
                                                               ↓
出力: u32 ← String ← bytes ← [C FFI] ← ByteArray ← String ← Nat
```

**各ステップの詳細:**

```
Rust main()
  │  env::read::<u32>()              zkVM 入力から u32 を読み取り
  │  input.to_string().into_bytes()  u32 → String → Vec<u8>
  │
  ├─► C: lean_risc0_main()
  │     byte_array_from_c()          バイト単位ループコピー → Lean ByteArray
  │
  │   ├─► Lean: risc0_main()
  │   │     String.fromUTF8!         ByteArray → String（UTF-8 バリデーション）
  │   │     String.toNat!            String → Nat（10進数パース）
  │   │     sum n                    実際の計算
  │   │     toString result          Nat → String（10進数フォーマット）
  │   │     String.toUTF8            String → ByteArray
  │   │
  │     c_from_byte_array()          バイト単位ループコピー + malloc → char*
  │
  │  String::from_utf8()             bytes → String
  │  .parse::<u32>()                 String → u32
  │  env::commit(&value)             zkVM ジャーナルに u32 を書き込み
```

Rust ゲストは `env::read()`/`env::commit()` で `u32` を直接読み書きするため、
全てのマーシャリングが不要となる。

### 第4層: ELF サイズ → ページングコスト

RISC Zero zkVM ではメモリページの読み込みにサイクルコストが直接発生する
（prove モードの `paging_cycles`）。19.2 倍大きい ELF は大幅に多くのページ
読み込みを必要とする。

**ELF 構成（Lean ゲスト）:**

| コンポーネント | ソースサイズ | 備考 |
|--------------|------------|------|
| `libInit.a` | 23 MB | リンカが未使用シンボルを除去するが、初期化コードは残る |
| `libLean.a` | 944 KB | ランタイム: GC、参照カウント、メモリ管理 |
| libc + libstdc++ | 可変 | Lean ランタイムが必要とする |
| `libGuest.a` | 9.6 KB | ユーザーコード |
| Rust zkVM ランタイム | ~270 KB | Rust ゲストと共通 |
| **最終 ELF** | **5.1 MB** | リンカのデッドコード除去後 |

Rust ゲストの ELF（270 KB）はほぼ zkVM ランタイムのみで構成され、
`sum` 関数のサイズ増加は無視できる程度である。

## 定量サマリ

| 要因 | 推定サイクル | 寄与率 | 根拠 |
|------|------------|--------|------|
| Init 初期化（392 モジュール） | ~4,100,000 | **~99%** | 全 N で一定の基底サイクル |
| Nat ヒープ/RC オーバーヘッド | ~50/再帰 | N 依存 | N=10 と N=100 の差分 |
| データマーシャリング | ~数百 | <0.1% | 文字列変換のみ |
| ELF ページング | prove モードで顕在化 | execute では間接的 | 19.2x のサイズ差 |

## 改善の方向性

以下は実装計画ではなく、調査から特定された方向性である。

1. **Init インポートの排除** — `import Init` を避け、必要な定義のみ手動で
   用意する。`sum-example` ブランチがこのアプローチを実証しており、
   証明時間を ~13 分から数秒に短縮している。

2. **Nat の代わりに UInt32 を使用** — `Guest/Basic.lean` で `UInt32` を使えば
   ヒープ割り当てが不要になる。`UInt32` はアンボックス化された機械整数に
   コンパイルされる。

3. **マーシャリングの簡素化** — String 経由ではなくバイナリエンコーディングで
   `u32` を直接渡す。往復12段階の変換を排除できる。

4. **Init の遅延/部分初期化** — モジュールのオンデマンド初期化をサポートするには
   Lean コンパイラレベルの変更が必要。現行ツールチェーンでは実現困難。

## 再現手順

```bash
# 両ゲストをビルド
just build

# 実行ベンチマーク
just bench-execute

# 証明ベンチマーク（大幅に遅い）
just bench-prove
```

## 参照ファイル

| ファイル | 説明 |
|---------|------|
| `guest/Guest/Basic.lean` | Lean ビジネスロジック（`sum` 関数） |
| `guest/Guest.lean` | Lean エントリポイント（`@[export risc0_main]`） |
| `guest_build/risc0_ir/Guest.c` | コンパイル済み C IR: `initialize_Guest`、`initialize_Init` 呼び出し |
| `guest_build/risc0_ir/Guest/Basic.c` | コンパイル済み C IR: Nat ヒープ操作を伴う `l_sum` |
| `methods/guest/risc0_lean.c` | C ラッパー: ランタイム初期化 + データマーシャリング |
| `methods/guest/src/main.rs` | Rust ゲスト（Lean）: `lean_risc0_main` への FFI 呼び出し |
| `methods/guest-rust/src/main.rs` | Rust ゲスト（純粋）: 直接的な `sum` 実装 |
| `methods/guest/shims.c` | zkVM シム: 64 MB ヒープの `_sbrk` |
| `methods/guest/build.rs` | リンカ設定: libInit.a、libLean.a、libGuest.a |
| `host/src/bin/benchmark.rs` | ベンチマークハーネス: サイクル計測 + 比較 |
