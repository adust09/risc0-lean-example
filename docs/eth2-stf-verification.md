# ETH2 State Transition Function — zkVM 検証結果

## 結論

Ethereum Consensus Layer（Beacon Chain）の state transition function を Lean 4 で実装し、RISC Zero zkVM 上で Rust 実装と比較検証した。

Lean で書いた ETH2 STF は zkVM 上で正しく動作し、Rust 実装とバイト単位で完全に同一の出力を生成する。ただし Init ライブラリの初期化は必須である。Init をスキップすると、`Array`・`String`・`ByteArray`・`default` 等の closed term が BSS 上で NULL のまま残り、NULL dereference でクラッシュする。`UInt32` 等のアンボックス型のみで完結する sum 関数とは本質的に異なり、実用的な STF では Init を避けられない。また Init 初期化の過程で `initialize_Init_Data` が zkVM 上で失敗するため、事前に呼び出して `_G_initialized` フラグをセットするワークアラウンドが必要になる。

パフォーマンス面では、Lean の zkVM サイクル数は Rust の 2.1x〜2.4x（バリデータ数に依存）、ELF サイズは 17.7x となる。サイクル数オーバーヘッドの主因は Init の固定コスト（~15M cycles）と、永続データ構造の参照カウント操作である。

---

## 検証構成とベンチマーク結果

3つの guest 構成で比較した。テスト入力は slot 100 → 101 の 1-slot advance（エポック境界なし）、暗号プリミティブは全てスタブ。

| 構成 | 説明 |
|------|------|
| **Lean (no-init)** | `risc0_main_eth2()` を直接呼出。Init 初期化をスキップ |
| **Lean (init)** | Init_Data ワークアラウンド適用後に STF 呼出 |
| **Rust** | 同等ロジックの Pure Rust 実装（ベースライン） |

計測条件: `--suite eth2 --mode execute`（execute モード、`RISC0_DEV_MODE=1`）。

| Guest | N=10 | N=100 | ELF サイズ |
|-------|------|-------|-----------|
| Lean (no-init) | CRASHED | CRASHED | 2.6 MB |
| Lean (init) | 26,148,291 cycles / 29 seg | 35,281,299 cycles / 38 seg | 6.6 MB |
| Rust | 12,491,509 cycles / 13 seg | 14,446,747 cycles / 15 seg | 373 KB |
| **Lean/Rust 比率** | **2.1x** / 2.2x | **2.4x** / 2.5x | **17.7x** |

Lean (init) と Rust の出力はバイト単位で一致した（N=10: 78,746 B、N=100: 91,976 B）。

参照仕様: [eth2book](https://eth2book.info/latest/part3/transition/) / [ethereum/consensus-specs](https://github.com/ethereum/consensus-specs)

---

## Init スキップ実験の詳細

Init なし構成は以下のエラーでクラッシュする。

```
Invalid trap address: 0x00000000, cause: LoadAccessFault(0x00000008)
```

Lean コンパイラが生成する C IR では、`default` 値・空配列 `#[]`・リテラル `ByteArray.mk #[0xFF]` 等が closed term として BSS セグメントに配置される。`initialize_Guest()` がこれらを実行時に初期化するが、Init なしでは全て NULL のまま残り、アクセス時に NULL dereference が発生する。

クラッシュの根本原因を特定するため、C ラッパーを段階的に修正してバイナリサーチを行った。

| テスト | 内容 | 結果 |
|--------|------|------|
| 1 | Init のみ、静的バッファ返却 | PASS (15.8M cycles) |
| 2 | Init + `risc0_main(10)` 呼出 | PASS |
| 3 | Init + `risc0_main_eth2(empty)` + 結果アクセス | CRASH |
| 3b | 同上、結果無視 | PASS |
| 5 | 戻り値を raw ポインタとして出力 | **戻り値 = NULL** |
| 8 | closed term アドレスのメモリ読取 | **Guest 初期化後も NULL** |
| 9 | 初期化ステップの診断 | **`initialize_Init` が失敗** |
| 12 | Init サブモジュールを個別テスト | **`initialize_Init_Data` が失敗** |
| 13 | Data を先行呼出 → Init → Guest | **全て成功** |
| 15 | ワークアラウンド + 実データ | **STF 実行成功** |

失敗の原因は `initialize_Init_Data()` にある。Init_Data の初期化パスが libc のファイル操作を呼び出すが、zkVM の `shims.c` は全ファイル操作で `-1` を返す。一方 `strerror(0)` は "success" を返すため（errno が 0 のまま）、Init_Data はエラーコード 0 を「失敗」と判定し、エラーメッセージ `"success (error code: 0)"` を返す。

ワークアラウンドとして `initialize_Init_Data()` を先に呼ぶ。この呼出自体は失敗するが、内部的に `_G_initialized = true` フラグがセットされる。その後 `initialize_Init()` を呼ぶと、Init_Data は「既に初期化済み」としてスキップされ、全体が成功する。

```c
// methods/guest-eth2-init/risc0_lean.c
lean_initialize_runtime_module(lean_io_mk_world());  // (1) ランタイム初期化
initialize_Init_Data(1, lean_io_mk_world());         // (2) 失敗するが _G_initialized=true をセット
initialize_Init(1, lean_io_mk_world());              // (3) Data をスキップして成功
initialize_Guest(1, lean_io_mk_world());             // (4) 全 closed term を初期化
```

---

## オーバーヘッド分析

Lean/Rust のサイクル数比率は N=10 で 2.1x、N=100 で 2.4x と増加する。主な要因は 3 つある。

1. **Init 固定コスト（~15M cycles）** — 392 モジュールの初期化。N=10 では全体の約 57% を占め、入力サイズによらず一定
2. **永続データ構造のコスト** — Lean の `Array.set!` は参照カウントが 1 でない場合にコピーが発生する。バリデータ数が増えるほど影響が増大し、N=10 → N=100 で比率が 2.1x → 2.4x に上昇する
3. **参照カウント操作** — `{ state with ... }` による構造体更新時の RC increment/decrement

ELF サイズの 17.7x は主に Init ライブラリの静的リンクに起因する。内訳は Init ライブラリ ~4.0 MB、Lean ランタイム ~1.0 MB、Guest コード ~2.5 MB、libc/libstdc++ ~0.1 MB で合計 6.6 MB。Rust は zkVM ランタイムのみで 373 KB。

---

## 実装概要

Lean STF は `guest/Guest/Eth2/` 以下の 19 ファイルに実装した。型定義は Altair/Bellatrix 仕様に準拠（`Slot = UInt64`, `Gwei = UInt64`, `Root = ByteArray` 等）し、Epoch 処理（12 sub-functions）と Block 処理（header → randao → eth1_data → operations → sync_aggregate）を仕様順に実装した。暗号プリミティブ（`hash_tree_root`、BLS 検証）は全てスタブ、proposer 選出は `slot % active_validator_count`（RANDAO シャッフルなし）で簡略化している。

```
guest/Guest/Eth2/
  Types.lean, Constants.lean, Crypto.lean（スタブ）
  Containers.lean, Helpers.lean, Serialize.lean, Decode.lean
  Transition/
    StateTransition.lean        -- state_transition, process_slots
    Epoch.lean                  -- process_epoch（12 sub-functions）
    Block.lean                  -- process_block
    Block/{Header,Randao,Eth1Data,Operations,SyncAggregate}.lean
```

エントリポイントは `@[export risc0_main_eth2]` で C FFI にエクスポートされ、`ByteArray → ByteArray` のインターフェースで呼び出される。成功時はシリアライズされた post-state BeaconState を、失敗時は `0xFD` + UTF-8 エラーメッセージを返す。

FFI パイプラインは Rust guest → C wrapper（Init_Data workaround + `initialize_Guest`）→ Lean FFI → `Guest.lean`（decode → stateTransition → serialize）の順で処理される。guest crate は `methods/guest-eth2-init/`（動作する構成）、`methods/guest-eth2-noinit/`（Init スキップ実験の対照群）、`methods/guest-rust-eth2/`（Pure Rust ベースライン）の 3 つ。

---

## 今後の課題

- 暗号プリミティブの実装（`hash_tree_root`, BLS 検証）
- RANDAO ベースの proposer 選出
- エポック境界を跨ぐテストケース
- より大規模なバリデータセット（1,000+）での計測
- Init 固定コスト削減の調査（不要モジュールの除外等）

## 再現手順

```bash
export RISC0_TOOLCHAIN_PATH="$HOME/.risc0/toolchains/v2024.1.5-cpp-aarch64-apple-darwin/riscv32im-osx-arm64"
export LEAN_RISC0_PATH="$HOME/.lean-risc0"

just clean && just build
just bench-eth2-execute

# Or manually:
RISC0_DEV_MODE=1 cargo run --release --bin benchmark -- \
  --suite eth2 --mode execute --inputs 10,100 --guest all
```
