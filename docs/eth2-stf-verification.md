# ETH2 State Transition — zkVM 検証結果

Ethereum Consensus Layer (Beacon Chain) の state transition function を Lean 4 で仕様に忠実に実装し、RISC Zero zkVM 上で実行した検証結果。

## 検証構成

| Guest | 言語 | Init | 説明 |
|-------|------|------|------|
| **Lean (no-init)** | Lean 4 | なし | `risc0_main_eth2()` を直接呼出。Init 初期化をスキップ |
| **Lean (init)** | Lean 4 | あり | Init_Data ワークアラウンド + `initialize_Guest()` 後に呼出 |
| **Rust** | Rust | — | 同等ロジックの Pure Rust 実装。ベースライン |

テスト入力: slot 100 → 101 の 1-slot advance（エポック境界なし）。暗号プリミティブはスタブ。

## 計測結果

`--suite eth2 --mode execute`（execute モード、RISC0_DEV_MODE=1）。

| Guest | N=10 | N=100 |
|-------|------|-------|
| Lean (no-init) | CRASHED | CRASHED |
| Lean (init) | 26,148,291 cycles / 29 seg | 35,281,299 cycles / 38 seg |
| Rust | 12,491,509 cycles / 13 seg | 14,446,747 cycles / 15 seg |
| **Ratio (Lean/Rust)** | **2.1x** / 2.2x | **2.4x** / 2.5x |

| | Lean (no-init) | Lean (init) | Rust |
|--|-----:|-----:|-----:|
| ELF サイズ | 2.6 MB | 6.6 MB | 373 KB |
| 出力 (N=10) | CRASHED | 78,746 B | 78,746 B |
| 出力 (N=100) | CRASHED | 91,976 B | 91,976 B |

**重要**: Lean (init) と Rust の出力はバイト単位で一致。意味論的に同等な STF が動作している。

## Init スキップ実験の結果

### Lean (no-init) — CRASHED

```
Invalid trap address: 0x00000000, cause: LoadAccessFault(0x00000008)
```

**原因**: Lean コンパイラが生成する C IR では、`default` 値・空配列 `#[]`・リテラル `ByteArray.mk #[0xFF]` 等が「closed term」として BSS セグメントに配置される。これらは `initialize_Guest()` 中に初期化されるが、Init なしでは全て NULL のまま。`risc0_main_eth2()` がこれらにアクセスした時点で NULL dereference → LoadAccessFault。

**結論**: Array、String、ByteArray、default 等の Lean 標準機能を使う限り、**Init は必須**。UInt32 等のアンボックス型のみを使う sum 関数とは異なり、ETH2 STF では回避不可能。

### initialize_Init_Data の失敗

`initialize_Init()` → `initialize_Init_Data()` の呼出で zkVM 上で失敗する。

**原因**: Init_Data の初期化パスが libc のファイル操作を呼び出す。zkVM の shims.c では全ファイル操作が `-1` を返すが、`strerror(0)` は "success" を返す。Init_Data はエラーコード 0 を「失敗」と判定し、エラーメッセージ "success (error code: 0)" を返す。

**ワークアラウンド**: `initialize_Init_Data(1, lean_io_mk_world())` を先に呼ぶ。この呼出は失敗するが、内部的に `_G_initialized = true` フラグをセットする。その後 `initialize_Init()` を呼ぶと、Init_Data は「既に初期化済み」としてスキップされ、初期化全体が成功する。

```c
// risc0_lean.c (Init ワークアラウンド)
lean_initialize_runtime_module(lean_io_mk_world());  // (1) runtime
initialize_Init_Data(1, lean_io_mk_world());         // (2) fails, but sets flag
initialize_Init(1, lean_io_mk_world());              // (3) succeeds (skips Data)
initialize_Guest(1, lean_io_mk_world());             // (4) succeeds
```

## Lean STF 実装

### モジュール構成

```
guest/Guest/Eth2/
  Types.lean          -- Slot, Epoch, Gwei, Root 等
  Constants.lean      -- SLOTS_PER_EPOCH, MAX_EFFECTIVE_BALANCE 等
  Crypto.lean         -- hashTreeRoot, blsVerify (スタブ)
  Containers.lean     -- BeaconState, BeaconBlock, Validator 等
  Helpers.lean        -- getCurrentEpoch, getBaseReward, isActiveValidator 等
  Serialize.lean      -- BeaconState ↔ ByteArray 変換
  Decode.lean         -- ByteArray → 型 デシリアライゼーション
  Transition/
    StateTransition.lean  -- state_transition, process_slots
    Epoch.lean            -- process_epoch (12 sub-functions)
    Block.lean            -- process_block
    Block/Header.lean     -- process_block_header
    Block/Randao.lean     -- process_randao
    Block/Eth1Data.lean   -- process_eth1_data
    Block/Operations.lean -- process_operations
    Block/SyncAggregate.lean
```

### 仕様との対応

- 型定義: Altair/Bellatrix 仕様に準拠（`Slot = UInt64`, `Gwei = UInt64`, `Root = ByteArray` 等）
- Epoch 処理: 12 sub-functions を仕様順に実行
- Block 処理: header → randao → eth1_data → operations → sync_aggregate
- 暗号プリミティブ: 全てスタブ（`hashTreeRoot → 固定値`, `blsVerify → true`）
- proposer 選出: `slot % active_validator_count`（RANDAO シャッフルなし）

## 考察

### オーバーヘッド分析

Lean/Rust 比率が N=10 で 2.1x、N=100 で 2.4x と増加する。

要因:
1. **Init 固定コスト**: ~15M cycles（sum benchmark から推定）。N=10 では支配的
2. **Array 操作**: Lean の永続データ構造は `Array.set!` でコピーが発生する場合がある。バリデータ数が増えるとこの影響が増大
3. **参照カウント**: Lean ランタイムの RC 操作。特に構造体の更新（`{ state with ... }`）で発生

### ELF サイズ

Lean (init) の 6.6 MB は主に Init ライブラリ（392 モジュール）の静的リンクに起因。Lean (no-init) の 2.6 MB は Init なしだが Guest コード + Lean ランタイム + libc。Rust の 373 KB は zkVM ランタイムのみ。

## 再現手順

```bash
export RISC0_TOOLCHAIN_PATH="$HOME/.risc0/toolchains/v2024.1.5-cpp-aarch64-apple-darwin/riscv32im-osx-arm64"
export LEAN_RISC0_PATH="$HOME/.lean-risc0"

just clean && just build
just bench-eth2-execute
```
