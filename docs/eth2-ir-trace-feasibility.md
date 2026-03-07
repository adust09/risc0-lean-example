# ETH2 前提: Lean IR トレース検証アプローチのフィジビリティ検証

## 目的

Lean4 を C に落として zkVM ゲストで直接実行する現在方式は、Init 初期化コストが大きく改善余地が小さい。  
そこで、**Lean IR の実行トレースを zkVM で検証する方式**（`ir_interpreter.cpp` 参考）の実用性を、ETH2 state transition を前提に評価する。

検証日: **2026-03-06**

---

## 前提と検証スコープ

- 対象実装はこのリポジトリの ETH2 STF（暗号はスタブ）
- 比較対象:
  - Lean guest (no-init)
  - Lean guest (init)
  - Rust guest
- 計測モード: execute（`RISC0_DEV_MODE=1`）
- 入力の `N` は **validator 数 (`num_validators`)**
  - `N` は Lean 関数の直接引数ではなく、host 側で生成する `BeaconState` のサイズを決めるベンチ用パラメータ

関連箇所:
- benchmark 入力定義: `host/src/bin/benchmark.rs`
- Lean ETH2 エントリポイント: `guest/Guest.lean`

---

## 実測結果

コマンド:

```bash
LEAN_RISC0_PATH="$HOME/.lean-risc0" \
RISC0_TOOLCHAIN_PATH="$HOME/.risc0/toolchains/v2024.1.5-cpp-aarch64-apple-darwin/riscv32im-osx-arm64" \
RISC0_DEV_MODE=1 \
cargo run --release --bin benchmark -- --suite eth2 --mode execute --inputs 1,10,100,1000 --guest all
```

| N (= validators) | Lean(no-init) | Lean(init) User Cycles | Rust User Cycles | Lean(init)/Rust |
|---:|---|---:|---:|---:|
| 1 | CRASH (`LoadAccessFault`) | 25,159,050 | 12,276,996 | 2.0x |
| 10 | CRASH (`LoadAccessFault`) | 26,148,291 | 12,491,509 | 2.1x |
| 100 | CRASH (`LoadAccessFault`) | 35,281,299 | 14,446,747 | 2.4x |
| 1000 | CRASH (`LoadAccessFault`) | 130,527,911 | 35,237,893 | 3.7x |

観測:
- no-init は ETH2 では全ケース失敗（既存調査どおり）
- Lean(init) は Rust 比で入力が増えるほど不利になる（2.0x -> 3.7x）

---

## なぜ validator 数が STF コストに効くか

ETH2 STF は validator 集合を直接走査・更新するため、validator 数 `N` が計算量に直結する。

主因:

1. 状態デコード/エンコードが N に比例
- `validators`, `balances`, participation flags, `inactivityScores` など可変長配列を処理
- 参照: `guest/Guest/Eth2/Serialize.lean`

2. proposer/active validator 計算が N 走査
- `getActiveValidatorIndices` は validators を全走査
- 参照: `guest/Guest/Eth2/Helpers.lean`

3. epoch 処理の各サブ関数が N 依存ループ
- rewards/penalties, inactivity, effective balance, registry updates など
- 参照:
  - `guest/Guest/Eth2/Transition/Epoch/RewardsAndPenalties.lean`
  - `guest/Guest/Eth2/Transition/Epoch/InactivityUpdates.lean`
  - `guest/Guest/Eth2/Transition/Epoch/EffectiveBalances.lean`
  - `guest/Guest/Eth2/Transition/Epoch/RegistryUpdates.lean`

補足:
- 今回のベンチ入力（slot 100 -> 101）は epoch 境界を跨がないため、主に decode/encode と block 側コストが支配的。
- それでも N 増加に伴う差分コストは明確に増えている。

---

## IR トレース検証方式の評価（ETH2 前提）

### 結論

**フルスコープ（ETH2 全体）の IR トレース検証は、現時点では実用性が低い。**

理由:

1. IR 実行意味論の再現コストが高い
- Lean IR は `inc/dec/del/reset/reuse/case/jmp` 等を含み、RC とヒープ更新を厳密再現する必要がある
- 参照:
  - IR 命令定義: Lean `src/Lean/Compiler/IR/Basic.lean`
  - 参照実装: `src/library/ir_interpreter.cpp`

2. 参照実装はホスト依存機能を前提
- `dlsym/GetProcAddress` ベースの symbol 解決など、zkVM へそのまま載せられない部分が多い

3. トレース入力そのもののコスト
- zkVM では proving cost は cycle 数に比例し、入力/メモリアクセスにもコストが乗る
- 大規模トレースを投入すると、検証ロジック以前に I/O とページングの負担が大きい
- 参照: RISC Zero Guest Optimization Guide

---

## 実用的な提案

1. 本番経路は当面 Rust guest を維持
- 証明コスト最小化を優先

2. Lean は仕様源 + 差分検証に使う
- Lean 実装と Rust 実装の出力一致テストを継続

3. IR トレース方式は限定 PoC で段階評価
- 対象を `processSlots` 等の部分関数に限定
- 最小 opcode セットだけで verifier 実装
- 先に Go/No-Go 指標を固定（例: cycles/trace size 上限）

4. 指標未達ならフル ETH2 展開は中止
- 早期に技術的撤退ラインを設定

---

## 参照

- Lean IR interpreter 実装  
  https://github.com/leanprover/lean4/blob/master/src/library/ir_interpreter.cpp
- Lean IR 命令定義  
  https://github.com/leanprover/lean4/blob/master/src/Lean/Compiler/IR/Basic.lean
- RISC Zero Guest Optimization Guide  
  https://github.com/risc0/risc0/blob/main/website/api/zkvm/optimization.md
- 本リポジトリ既存検証  
  `docs/eth2-stf-verification.md`

