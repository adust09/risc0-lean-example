# Lean ゲスト オーバーヘッド分析

RISC Zero zkVM 上で Lean 4 コードを実行した際のパフォーマンスオーバーヘッドを、
同等の純 Rust 実装と比較して分析する。

## ベンチマーク概要

**テスト関数:** `sum(n) = if n == 0 then 0 else (n + sum(n-1)) &&& 0xFFFF`

両ゲストは同一のアルゴリズムを計算する。Lean ゲストは Lean → C → RISC-V の
フルパイプラインを経由し、Rust ゲストは直接実装である。

**方式:** UInt32 直接 FFI（Init ライブラリ初期化をスキップ）

### 実行モード結果（証明なし）

| ゲスト | N | ユーザーサイクル | セグメント | 実行時間 |
|--------|------:|-----------:|---------:|---------:|
| Lean   |    10 |      3,596 |        1 |   9.8s   |
| Rust   |    10 |      3,736 |        1 |    37ms  |
| **倍率** | | **1.0x** | **1.0x** | **~265x** |
| | | | | |
| Lean   |   100 |      4,946 |        1 |   9.8s   |
| Rust   |   100 |      5,176 |        1 |    38ms  |
| **倍率** | | **1.0x** | **1.0x** | **~258x** |
| | | | | |
| Lean   | 1,000 |     18,446 |        1 |  11.5s   |
| Rust   | 1,000 |     19,576 |        1 |    39ms  |
| **倍率** | | **0.9x** | **1.0x** | **~294x** |
| | | | | |
| Lean   | 5,000 |     78,446 |        1 |   9.8s   |
| Rust   | 5,000 |     83,576 |        1 |    53ms  |
| **倍率** | | **0.9x** | **1.0x** | **~186x** |
| | | | | |
| Lean   | 10,000 |   153,446 |        1 |  10.4s   |
| Rust   | 10,000 |   163,576 |        1 |    58ms  |
| **倍率** | | **0.9x** | **1.0x** | **~179x** |

### ELF バイナリサイズ

| | Lean | Rust | 倍率 |
|--|-----:|-----:|-----:|
| ELF サイズ | 1,536,296 bytes (1.5 MB) | 276,472 bytes (270 KB) | **5.6x** |

## 分析

### サイクル数: Lean ≈ Rust（倍率 0.9x〜1.0x）

UInt32 直接 FFI 方式により、Lean と Rust のサイクル数はほぼ同一となった。
Lean が若干少ないサイクルで実行されるケースもある（0.9x）。

**理由:** Lean の `UInt32` はアンボックス化された機械整数にコンパイルされる。
生成される C IR は Rust の実装とほぼ等価である。

**Lean C IR** (`guest_build/risc0_ir/Guest/Basic.c`):

```c
LEAN_EXPORT uint32_t l_sum(uint32_t x_1) {
    x_2 = 0;
    x_3 = lean_uint32_dec_eq(x_1, x_2);
    if (x_3 == 0) {
        x_5 = lean_uint32_sub(x_1, 1);
        x_6 = l_sum(x_5);
        x_7 = lean_uint32_add(x_1, x_6);
        x_9 = lean_uint32_land(x_7, 65535);
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

ヒープ割り当て: 0回、参照カウント: 0回、型変換: 0回。
Lean の `lean_uint32_*` 関数はインラインの機械整数操作であり、
Rust のネイティブ演算と同等のパフォーマンスを示す。

### 壁時計時間の差異

サイクル数が同一であるにもかかわらず、壁時計時間には大きな差がある
（~10s vs ~50ms、~200x）。これは以下の理由による:

1. **ELF サイズ差（5.6x）:** Lean ゲストの ELF は 1.5 MB（Init ライブラリが
   リンクされているが実行されない）。Rust は 270 KB。ホスト側の ELF ロードと
   セットアップにかかる時間が異なる。

2. **zkVM サイクル数には影響なし:** 壁時計時間はホスト側のオーバーヘッドであり、
   zkVM 証明コストを反映しない。証明コストはサイクル数に比例するため、
   Lean と Rust の証明コストはほぼ同一である。

### FFI アーキテクチャ

```
Rust guest → extern "C" lean_simple_risc0_main(u32) → risc0_main(uint32_t) → l_sum(uint32_t)
```

Init ランタイム初期化 (`lean_initialize_runtime_module()`, `initialize_Guest()`)
をスキップし、`risc0_main` を直接呼び出す。UInt32 は C ABI で `uint32_t` として
直接渡されるため、マーシャリングが不要。

## 参考: Init ライブラリ初期化ありの結果

以下は Init ライブラリを初期化する旧方式（Nat + ByteArray FFI）の結果である。

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

**旧方式のオーバーヘッド要因:**
- Init ライブラリ初期化: ~4,100,000 サイクル（全体の ~99%）
- Nat ヒープ割り当て + 参照カウント: ~50 サイクル/再帰
- データマーシャリング: u32 → String → bytes → ByteArray → Nat（往復12段階）
- ELF サイズ: 5.1 MB（19.2x）

## 再現手順

```bash
# 環境変数の設定
export RISC0_TOOLCHAIN_PATH="$HOME/.risc0/toolchains/v2024.1.5-cpp-aarch64-apple-darwin/riscv32im-osx-arm64"
export LEAN_RISC0_PATH="$HOME/.lean-risc0"

# 両ゲストをビルド
just build

# 動作確認
target/release/host 100  # → 5050

# 実行ベンチマーク
just bench-execute

# 証明ベンチマーク（大幅に遅い）
just bench-prove
```

## 参照ファイル

| ファイル | 説明 |
|---------|------|
| `guest/Guest/Basic.lean` | Lean ビジネスロジック（`sum` 関数、UInt32） |
| `guest/Guest.lean` | Lean エントリポイント（`@[export risc0_main]`、UInt32 直接） |
| `guest_build/risc0_ir/Guest/Basic.c` | コンパイル済み C IR: `uint32_t l_sum(uint32_t)` |
| `methods/guest/risc0_lean.c` | C ラッパー: Init 初期化なし、直接 FFI |
| `methods/guest/src/main.rs` | Rust ゲスト（Lean）: `lean_simple_risc0_main` への直接 FFI |
| `methods/guest-rust/src/main.rs` | Rust ゲスト（純粋）: 直接的な `sum` 実装 |
| `methods/guest/shims.c` | zkVM シム: 64 MB ヒープの `_sbrk` |
| `methods/guest/build.rs` | リンカ設定: libInit.a、libLean.a、libGuest.a |
| `host/src/bin/benchmark.rs` | ベンチマークハーネス: サイクル計測 + 比較 |
