# Lean vs Rust zkVM パフォーマンス比較

## 目的

RISC Zero zkVM 上で同一アルゴリズムを Lean 4 と Rust で実装し、zkVM サイクル数（= 証明コスト）を比較する。

## 計測対象

再帰的な `sum` 関数。16ビットマスク付きで Lean/Rust 間の出力一致を保証する。

```
sum(n) = if n == 0 then 0 else (n + sum(n - 1)) &&& 0xFFFF
```

**Lean 実装** (`guest/Guest/Basic.lean`):
```lean
partial def sum (n : UInt32) : UInt32 :=
  if n == 0 then 0 else (n + sum (n - 1)) &&& 0xFFFF
```

**Rust 実装** (`methods/guest-rust/src/main.rs`):
```rust
fn sum(n: u32) -> u32 {
    if n == 0 { 0 } else { (n + sum(n - 1)) & 0xFFFF }
}
```

## パイプライン

Lean ゲストは Lean → C → RISC-V の変換パイプラインを経由する。Rust ゲストは直接コンパイルされる。

```
Lean:  Lean 4 → lake build → C IR → CMake (riscv32-gcc) → libGuest.a → Rust FFI → zkVM ELF
Rust:  Rust → cargo build (risc0-zkvm) → zkVM ELF
```

呼び出しチェーン:

```
[Rust ゲスト]                           [Lean ゲスト]
env::read::<u32>()                      env::read::<u32>()
sum(input)                              lean_simple_risc0_main(input)  // C FFI
env::commit(&result)                      → risc0_main(uint32_t)       // Lean exported
                                            → l_sum(uint32_t)          // Lean compiled
                                        env::commit(&value)
```

## 計測結果

`just bench-execute`（execute モード、証明生成なし）で計測。3回実行の中央値。

| ゲスト | N | ユーザーサイクル | セグメント | 壁時計時間 |
|--------|------:|-----------:|---------:|---------:|
| Lean   |    10 |      3,596 |        1 |   9.8s   |
| Rust   |    10 |      3,736 |        1 |    37ms  |
| **比率** | | **0.96x** | **1.0x** | |
| | | | | |
| Lean   |   100 |      4,946 |        1 |   9.8s   |
| Rust   |   100 |      5,176 |        1 |    38ms  |
| **比率** | | **0.96x** | **1.0x** | |
| | | | | |
| Lean   | 1,000 |     18,446 |        1 |  11.5s   |
| Rust   | 1,000 |     19,576 |        1 |    39ms  |
| **比率** | | **0.94x** | **1.0x** | |
| | | | | |
| Lean   | 5,000 |     78,446 |        1 |   9.8s   |
| Rust   | 5,000 |     83,576 |        1 |    53ms  |
| **比率** | | **0.94x** | **1.0x** | |
| | | | | |
| Lean   | 10,000 |   153,446 |        1 |  10.4s   |
| Rust   | 10,000 |   163,576 |        1 |    58ms  |
| **比率** | | **0.94x** | **1.0x** | |

**比率は Lean/Rust。1.0 未満は Lean の方がサイクル数が少ないことを意味する。**

### ELF サイズ

| | Lean | Rust | 比率 |
|--|-----:|-----:|-----:|
| ELF | 1,536,296 bytes (1.5 MB) | 276,472 bytes (270 KB) | 5.6x |

## 考察

### サイクル数が同等である理由

Lean の `UInt32` 型はアンボックス化された機械整数としてコンパイルされる。生成される C コードは Rust と構造的に等価である。

Lean コンパイラが生成した C IR（`guest_build/risc0_ir/Guest/Basic.c`）:

```c
LEAN_EXPORT uint32_t l_sum(uint32_t x_1) {
  uint32_t x_2; uint8_t x_3;
  x_2 = 0;
  x_3 = lean_uint32_dec_eq(x_1, x_2);
  if (x_3 == 0) {
    uint32_t x_4; uint32_t x_5; uint32_t x_6; uint32_t x_7; uint32_t x_8; uint32_t x_9;
    x_4 = 1;
    x_5 = lean_uint32_sub(x_1, x_4);
    x_6 = l_sum(x_5);
    x_7 = lean_uint32_add(x_1, x_6);
    x_8 = 65535;
    x_9 = lean_uint32_land(x_7, x_8);
    return x_9;
  } else {
    return x_2;
  }
}
```

`lean_uint32_add` 等はインラインの機械整数演算であり、ヒープ割り当ても参照カウントも発生しない。

### 1再帰あたりのサイクル単価

N=10 と N=10,000 の差分から1再帰あたりのコストを算出:

| | Lean | Rust |
|--|-----:|-----:|
| 1再帰あたり | (153,446 - 3,596) / 9,990 ≈ **15.0** | (163,576 - 3,736) / 9,990 ≈ **16.0** |

Lean が Rust より約6%少ないサイクルで各再帰を実行している。N が大きくなるほどこの差が蓄積し、比率が 0.96x → 0.94x へ推移する。

### 壁時計時間がサイクル数と乖離する理由

壁時計時間は Lean ~10s / Rust ~40ms と約250倍の差がある。これはホスト側（zkVM の外）で発生する ELF ロード・セットアップ時間の差であり、ゲスト内の実行サイクル数とは無関係である。

要因:
- Lean ELF は 1.5 MB、Rust ELF は 270 KB（5.6倍）
- Init ライブラリはリンクされているが実行時には呼び出されない（サイズのみ影響）
- zkVM の証明コストはサイクル数に比例するため、**証明コストは Lean ≈ Rust**

### Init ライブラリを呼び出さない仕組み

C ラッパー (`methods/guest/risc0_lean.c`) が `risc0_main` を直接呼び出す:

```c
extern uint32_t risc0_main(uint32_t input);

uint32_t lean_simple_risc0_main(uint32_t n) {
    return risc0_main(n);
}
```

生成された C IR には `initialize_Init()` を呼ぶ `initialize_Guest()` 関数が存在するが、この関数を呼び出さないため Init の初期化コード（392モジュール、~410万サイクル）は一切実行されない。ただし `libInit.a` はリンカの依存解決で ELF に含まれる。

### 制約事項

Init ライブラリの初期化をスキップしているため、以下の Lean 機能は使用できない:

- `String`, `List`, `Array` 等の Init に依存する型
- `IO` モナドとそれに依存する機能
- `Nat`（ヒープ割り当てが必要）
- 文字列リテラル

使用可能なのは `UInt8`, `UInt16`, `UInt32`, `UInt64`, `Bool` 等のアンボックス型に限定される。

## 付録: 旧方式の結果

以前の方式では `Nat` + `ByteArray` FFI + Init 初期化を使用していた。

| ゲスト | N | ユーザーサイクル | セグメント |
|--------|------:|-----------:|---------:|
| Lean   |    10 |  4,119,771 |        5 |
| Rust   |    10 |      3,736 |        1 |
| **比率** | | **1,102.7x** | **5.0x** |
| | | | |
| Lean   |   100 |  4,124,252 |        5 |
| Rust   |   100 |      5,176 |        1 |
| **比率** | | **796.8x** | **5.0x** |
| | | | |
| Lean   | 1,000 |  4,161,410 |        6 |
| Rust   | 1,000 |     19,576 |        1 |
| **比率** | | **212.6x** | **6.0x** |

N=10 で 4,119,771 サイクル、N=100 で 4,124,252 サイクルと、入力サイズに関係なくほぼ一定の ~410万サイクルが消費されていた。これは `initialize_Init()` による 392 モジュールの初期化コストである。

旧方式と現方式の比較:

| | 旧方式 | 現方式 |
|--|:--|:--|
| Lean 型 | `Nat`（ヒープ割り当て） | `UInt32`（機械整数） |
| FFI | `ByteArray` マーシャリング（往復12段階） | `uint32_t` 直接（変換なし） |
| Init 初期化 | あり（~410万サイクル） | なし |
| Lean/Rust 比率 | ~1,000x | ~0.94x |
| ELF サイズ | 5.1 MB (19.2x) | 1.5 MB (5.6x) |
| セグメント数 | 5〜6 | 1 |

## 再現手順

```bash
export RISC0_TOOLCHAIN_PATH="$HOME/.risc0/toolchains/v2024.1.5-cpp-aarch64-apple-darwin/riscv32im-osx-arm64"
export LEAN_RISC0_PATH="$HOME/.lean-risc0"

just clean && just build
target/release/host 100   # → 5050
just bench-execute
```
