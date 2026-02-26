# Lean vs Rust zkVM パフォーマンス比較

RISC Zero zkVM 上で同一アルゴリズムを実装した3方式のゲストについて、zkVM サイクル数（= 証明コスト）を比較する。

計測対象は再帰的な `sum` 関数。16ビットマスクにより全方式で出力が一致することを保証する。

```
sum(n) = if n == 0 then 0 else (n + sum(n - 1)) &&& 0xFFFF
```

| 方式 | 言語 | 型 | FFI | Init 初期化 |
|------|------|-----|-----|-------------|
| **Lean (UInt32)** | Lean 4 | `UInt32`（機械整数） | `uint32_t` 直接 | なし |
| **Lean (Nat)** | Lean 4 | `Nat`（ヒープ割り当て） | `ByteArray` マーシャリング | あり（392モジュール） |
| **Rust** | Rust | `u32`（機械整数） | なし | なし |

各方式のトレードオフ:

- **Lean (UInt32)**: サイクル効率は Rust と同等。ただし Init を初期化しないため、`String`, `List`, `Array`, `IO`, `Nat` 等の Lean 標準機能は使用不可。`UInt8`〜`UInt64`, `Bool` 等のアンボックス型に限定される。
- **Lean (Nat)**: Lean の標準ライブラリを全て使用可能。ただし Init 初期化に ~410万サイクルの固定コストが発生し、`Nat` の各演算でヒープ割り当て・参照カウントが必要。
- **Rust**: ベースライン。zkVM ランタイムのみで追加のオーバーヘッドなし。

ビルドパイプライン:

```
Lean:  Lean 4 → lake build → C IR → CMake (riscv32-gcc) → libGuest.a → Rust FFI → zkVM ELF
Rust:  Rust → cargo build (risc0-zkvm) → zkVM ELF
```

## 計測結果

`just bench-execute`（execute モード、証明生成なし）。3回実行の中央値。比率は Lean/Rust。1.0 未満は Lean の方がサイクル数が少ないことを意味する。

| N | Lean (UInt32) | Lean (Nat) | Rust | UInt32/Rust | Nat/Rust |
|------:|----------:|----------:|----------:|-----:|-----:|
|    10 |     3,596 | 4,119,771 |     3,736 | 0.96x | 1,102.7x |
|   100 |     4,946 | 4,124,252 |     5,176 | 0.96x | 796.8x |
| 1,000 |    18,446 | 4,161,410 |    19,576 | 0.94x | 212.6x |
| 5,000 |    78,446 |         — |    83,576 | 0.94x | — |
|10,000 |   153,446 |         — |   163,576 | 0.94x | — |

| | Lean (UInt32) | Lean (Nat) | Rust |
|--|-----:|-----:|-----:|
| ELF サイズ | 1.5 MB (5.6x) | 5.1 MB (19.2x) | 270 KB |
| セグメント数 (N=1,000) | 1 | 6 | 1 |

## 考察

**Lean (UInt32) のサイクル効率** — Lean の `UInt32` はアンボックス化された機械整数にコンパイルされる。生成される C IR は Rust と構造的に等価で、ヒープ割り当ても参照カウントも発生しない。

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

1再帰あたりのサイクル単価（N=10 と N=10,000 の差分から算出）: Lean (UInt32) ≈ 15.0、Rust ≈ 16.0。Lean が約6%効率的であり、N が大きいほど比率が 0.96x → 0.94x に推移する。

**Lean (Nat) の固定コスト** — N=10 で 4,119,771 サイクル、N=100 で 4,124,252 サイクルと、入力サイズに関係なく ~410万サイクルの固定コストが観測される。これは `initialize_Init()` による 392 モジュールの初期化で、ランタイム起動時にヒープ上に静的変数を確保する処理である。加えて `Nat` 型はヒープ割り当てが必要なボックス化オブジェクトであり、1再帰あたり ~50 サイクル（Rust の ~16 の約3倍）。

**壁時計時間の乖離** — 全方式でサイクル数と壁時計時間に乖離がある。壁時計時間はホスト側の ELF ロード・セットアップ時間を含み、ゲスト内の実行サイクル数とは無関係である。zkVM の証明コストはサイクル数に比例するため、壁時計時間ではなくサイクル数が実質的なコスト指標となる。

**Init スキップの仕組み** — Lean (UInt32) 方式の C ラッパー (`methods/guest/risc0_lean.c`) は `risc0_main` を直接呼び出す。生成された C IR には `initialize_Init()` を呼ぶ `initialize_Guest()` が存在するが、この関数を呼び出さないため Init の初期化コードは実行されない。`libInit.a` はリンカの依存解決で ELF に含まれる（ELF サイズ差 5.6x の一因）。

## 再現手順

```bash
export RISC0_TOOLCHAIN_PATH="$HOME/.risc0/toolchains/v2024.1.5-cpp-aarch64-apple-darwin/riscv32im-osx-arm64"
export LEAN_RISC0_PATH="$HOME/.lean-risc0"

just clean && just build
target/release/host 100   # → 5050
just bench-execute
```
