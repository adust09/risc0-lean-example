# ETH2 Prerequisite: Feasibility Study of Lean IR Trace Verification Approach

## Purpose

The current approach of compiling Lean 4 to C and executing it directly in the zkVM guest incurs high Init initialization costs with limited room for improvement.
This document evaluates the practicality of an **alternative approach that verifies Lean IR execution traces within the zkVM** (referencing `ir_interpreter.cpp`), assuming an ETH2 state transition workload.

Date of evaluation: **2026-03-06**

---

## Scope and Assumptions

- Target implementation is the ETH2 STF in this repository (crypto is stubbed)
- Comparison targets:
  - Lean guest (no-init)
  - Lean guest (init)
  - Rust guest
- Measurement mode: execute (`RISC0_DEV_MODE=1`)
- `N` refers to **validator count (`num_validators`)**
  - `N` is not a direct argument to Lean functions; it is a benchmark parameter that determines the size of the `BeaconState` generated on the host side

Related code:
- Benchmark input definition: `host/src/bin/benchmark.rs`
- Lean ETH2 entry point: `guest/Guest.lean`

---

## Measured Results

Command:

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

Observations:
- no-init fails in all ETH2 cases (consistent with prior investigation)
- Lean(init) becomes increasingly disadvantaged relative to Rust as input grows (2.0x → 3.7x)

---

## Why Validator Count Affects STF Cost

The ETH2 STF directly iterates over and updates the validator set, so validator count `N` directly determines computational cost.

Main factors:

1. State decoding/encoding is proportional to N
- Processes variable-length arrays: `validators`, `balances`, participation flags, `inactivityScores`, etc.
- Reference: `guest/Guest/Eth2/Serialize.lean`

2. Proposer/active validator computation scans all N validators
- `getActiveValidatorIndices` performs a full scan of validators
- Reference: `guest/Guest/Eth2/Helpers.lean`

3. Each epoch processing sub-function has N-dependent loops
- rewards/penalties, inactivity, effective balance, registry updates, etc.
- References:
  - `guest/Guest/Eth2/Transition/Epoch/RewardsAndPenalties.lean`
  - `guest/Guest/Eth2/Transition/Epoch/InactivityUpdates.lean`
  - `guest/Guest/Eth2/Transition/Epoch/EffectiveBalances.lean`
  - `guest/Guest/Eth2/Transition/Epoch/RegistryUpdates.lean`

Note:
- The benchmark input (slot 100 → 101) does not cross an epoch boundary, so decode/encode and block-side costs are dominant.
- Even so, the incremental cost from increasing N is clearly visible.

---

## Evaluation of IR Trace Verification Approach (ETH2 Context)

### Conclusion

**Full-scope (entire ETH2) IR trace verification is not practical at this time.**

Reasons:

1. High cost of reproducing IR execution semantics
- Lean IR includes `inc/dec/del/reset/reuse/case/jmp` etc., requiring faithful reproduction of RC and heap updates
- References:
  - IR instruction definitions: Lean `src/Lean/Compiler/IR/Basic.lean`
  - Reference implementation: `src/library/ir_interpreter.cpp`

2. The reference implementation depends on host-specific functionality
- Symbol resolution via `dlsym/GetProcAddress` and other features that cannot be directly ported to the zkVM

3. Cost of the trace input itself
- In the zkVM, proving cost is proportional to cycle count, and input/memory access also incurs cost
- Feeding large traces results in significant I/O and paging overhead before any verification logic runs
- Reference: RISC Zero Guest Optimization Guide

---

## Practical Recommendations

1. Maintain Rust guest as the production path for now
- Prioritize minimizing proving cost

2. Use Lean as the specification source + differential verification
- Continue output-matching tests between the Lean and Rust implementations

3. Evaluate IR trace approach with a limited PoC in stages
- Restrict scope to partial functions such as `processSlots`
- Implement the verifier with a minimal opcode set only
- Establish Go/No-Go criteria first (e.g., cycles/trace size upper bounds)

4. Abandon full ETH2 deployment if criteria are not met
- Set a technical exit line early

---

## References

- Lean IR interpreter implementation
  https://github.com/leanprover/lean4/blob/master/src/library/ir_interpreter.cpp
- Lean IR instruction definitions
  https://github.com/leanprover/lean4/blob/master/src/Lean/Compiler/IR/Basic.lean
- RISC Zero Guest Optimization Guide
  https://github.com/risc0/risc0/blob/main/website/api/zkvm/optimization.md
- Prior verification in this repository
  `docs/eth2-stf-direct-execution.md`
