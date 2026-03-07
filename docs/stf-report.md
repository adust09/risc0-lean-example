# Lean 4 ETH2 State Transition Function on RISC Zero zkVM: A Comparative Study of Direct Execution and IR Trace Verification

## 1. Background

The Ethereum Consensus Layer (Beacon Chain) state transition function (STF) governs how the beacon state evolves slot by slot — processing block headers, attestations, sync committees, validator registry updates, and reward/penalty calculations. As the single authoritative transformation, the STF is a natural target for both formal verification and zero-knowledge proving.

Lean 4, a dependently typed programming language with a compiled runtime, offers a path toward specification-level implementations that can be both formally verified and executed. RISC Zero zkVM provides a general-purpose proving platform that executes RISC-V binaries and generates zero-knowledge proofs of correct execution. The combination — implementing the STF in Lean 4 and proving its execution inside RISC Zero zkVM — would allow a formally verified STF to produce cryptographic proofs of state transitions.

The central challenge is overhead. Lean 4 compiles through C intermediate representation (IR) and requires a runtime with reference counting, heap allocation, and a standard library initialization phase. These introduce cycle cost and binary size overhead relative to a native Rust implementation on the same zkVM. Two candidate approaches address this challenge differently:

- **Approach A (Direct Execution):** Compile Lean 4 to C IR, cross-compile to RISC-V, and execute directly inside the zkVM guest.
- **Approach B (IR Trace Verification):** Execute the Lean program on a host machine, capture the IR-level execution trace, and verify that trace inside the zkVM.

This report presents quantitative results for Approach A and a qualitative feasibility assessment for Approach B. The scope covers the Altair/Bellatrix specification with all cryptographic primitives stubbed and a simplified proposer selection (`slot % active_validator_count`, no RANDAO shuffle).

## 2. Approaches

### 2.1 Approach A: Direct Execution

**Pipeline.** The Lean 4 source is compiled to C IR via Lake, cross-compiled to RISC-V 32-bit object files via CMake, linked as `libGuest.a` into a Rust guest crate, and executed inside the zkVM:

```
Lean 4 → C IR (Lake) → RISC-V objects (CMake/riscv32-gcc) → libGuest.a → Rust guest (FFI) → zkVM ELF
```

**Init library requirement.** Lean's standard library places closed terms — `default` values, empty arrays `#[]`, literals such as `ByteArray.mk #[0xFF]` — in the BSS segment. These are initialized at runtime by `initialize_Guest()`, which transitively calls `initialize_Init()` to set up 392 modules. Without this initialization, all closed terms remain NULL, causing immediate crashes on access. Unlike a sum function operating solely on unboxed types (`UInt32`), a practical STF uses `Array`, `String`, `ByteArray`, and other boxed types that require Init.

**The Init_Data problem.** The standard initialization sequence fails on the zkVM because `initialize_Init_Data()` invokes libc file operations, and the zkVM's `shims.c` returns `-1` for all file I/O. However, since `errno` remains 0, `strerror(0)` returns `"success"`, and Init_Data interprets error code 0 as a failure, returning the error message `"success (error code: 0)"`. The workaround exploits the internal `_G_initialized` flag:

```c
// methods/guest-eth2-init/risc0_lean.c
lean_initialize_runtime_module(lean_io_mk_world());  // (1) Runtime initialization
initialize_Init_Data(1, lean_io_mk_world());         // (2) Fails but sets _G_initialized=true
initialize_Init(1, lean_io_mk_world());              // (3) Skips Data, succeeds
initialize_Guest(1, lean_io_mk_world());             // (4) Initializes all closed terms
```

**Implementation.** The Lean STF is implemented across 19 files under `guest/Guest/Eth2/`. Type definitions conform to the Altair/Bellatrix spec (`Slot = UInt64`, `Gwei = UInt64`, `Root = ByteArray`, etc.). Epoch processing (12 sub-functions) and block processing (header, randao, eth1_data, operations, sync_aggregate) follow spec order.

```
guest/Guest/Eth2/
  Types.lean, Constants.lean, Crypto.lean (stub)
  Containers.lean, Helpers.lean, Serialize.lean, Decode.lean
  Transition/
    StateTransition.lean        -- state_transition, process_slots
    Epoch.lean                  -- process_epoch (12 sub-functions)
    Block.lean                  -- process_block
    Block/{Header,Randao,Eth1Data,Operations,SyncAggregate}.lean
```

The entry point is exported via `@[export risc0_main_eth2]` with a `ByteArray → ByteArray` interface. On success, it returns the serialized post-state `BeaconState`; on failure, `0xFD` followed by a UTF-8 error message.

**Guest crate variants.** Three guest crates exist:
- `methods/guest-eth2-init/` — Lean guest with Init_Data workaround (working configuration)
- `methods/guest-eth2-noinit/` — Lean guest without Init (control group, crashes)
- `methods/guest-rust-eth2/` — Pure Rust implementation of the same algorithm (baseline)

### 2.2 Approach B: IR Trace Verification

**Concept.** Rather than compiling Lean to native code and executing inside the zkVM, this approach would execute the Lean program on the host, capture the Lean IR-level execution trace, and feed that trace into a zkVM guest that verifies each IR instruction was executed correctly. This avoids the Init library overhead entirely, since initialization occurs on the host outside the proof.

**Reference implementation.** Lean 4's `ir_interpreter.cpp` (in `src/library/`) provides a reference interpreter for Lean IR. The IR instruction set includes `inc`, `dec`, `del`, `reset`, `reuse`, `case`, `jmp`, and others, with semantics that faithfully track reference counts and heap state.

**Required semantics.** A zkVM-side verifier would need to reproduce:
- IR instruction dispatch for the full opcode set
- Reference counting operations (increment, decrement, deallocation)
- Heap allocation and management
- Constructor tagging and case dispatch

**Host-specific dependencies.** The reference interpreter relies on `dlsym` (Linux/macOS) and `GetProcAddress` (Windows) for dynamic symbol resolution of external functions, and other host-level features that cannot be directly ported to the zkVM environment.

**Status.** This approach was evaluated qualitatively only. No prototype was built and no quantitative measurements were taken.

## 3. Experimental Method

### 3.1 Guest Configurations

| Configuration | Description |
|---|---|
| **Lean (no-init)** | Direct call to `risc0_main_eth2()`. Skips Init initialization |
| **Lean (init)** | STF called after applying Init_Data workaround (4-step initialization) |
| **Rust** | Equivalent pure Rust implementation (baseline) |

### 3.2 Input Specification

The test input is a single-slot advance from slot 100 to slot 101 with no epoch boundary crossing. All cryptographic primitives are stubbed. The validator count `N` serves as the independent variable, controlling the size of the `BeaconState` generated on the host side. Tested values: N = 1, 10, 100, 1000.

### 3.3 Measurement

All measurements use execute mode (`--suite eth2 --mode execute`, `RISC0_DEV_MODE=1`), which counts cycles without generating proofs. The primary metric is **user cycle count**, as zkVM proving cost is directly proportional to cycles. Secondary metrics include segment count, ELF binary size, and output size.

For the Init skip experiment, the C wrapper was incrementally modified to isolate the crash root cause through a binary-search diagnostic sequence (Tests 1–15).

The IR trace verification approach (Approach B) was assessed qualitatively based on analysis of the Lean 4 IR interpreter source code and zkVM constraints. No prototype was built, and no quantitative data was collected.

### 3.4 Output Verification

The Lean (init) and Rust guests were verified to produce byte-identical output for each value of N, confirming functional equivalence of the two implementations.

### 3.5 Reproduction

```bash
export RISC0_TOOLCHAIN_PATH="$HOME/.risc0/toolchains/v2024.1.5-cpp-aarch64-apple-darwin/riscv32im-osx-arm64"
export LEAN_RISC0_PATH="$HOME/.lean-risc0"

just clean && just build
just bench-eth2-execute

# Or manually:
RISC0_DEV_MODE=1 cargo run --release --bin benchmark -- \
  --suite eth2 --mode execute --inputs 1,10,100,1000 --guest all
```

## 4. Experimental Results

### 4.1 Cycle Count Comparison

| N (validators) | Lean (no-init) | Lean (init) cycles / seg | Rust cycles / seg | Lean(init)/Rust | IR Trace Verification |
|--:|---|--:|--:|--:|---|
| 1 | CRASH (`LoadAccessFault`) | 25,159,050 / — | 12,276,996 / — | 2.0x | N/A — no prototype built |
| 10 | CRASH (`LoadAccessFault`) | 26,148,291 / 29 seg | 12,491,509 / 13 seg | 2.1x | N/A — no prototype built |
| 100 | CRASH (`LoadAccessFault`) | 35,281,299 / 38 seg | 14,446,747 / 15 seg | 2.4x | N/A — no prototype built |
| 1000 | CRASH (`LoadAccessFault`) | 130,527,911 / — | 35,237,893 / — | 3.7x | N/A — no prototype built |

### 4.2 ELF Size

| Guest | ELF Size |
|---|--:|
| Lean (init) | 6.6 MB |
| Rust | 373 KB |
| **Lean / Rust ratio** | **17.7x** |

ELF size breakdown for Lean (init): Init library ~4.0 MB, Lean runtime ~1.0 MB, guest code ~2.5 MB, libc/libstdc++ ~0.1 MB.

### 4.3 Output Correctness

Lean (init) and Rust outputs matched byte-for-byte at all tested values of N (N=10: 78,746 B; N=100: 91,976 B).

### 4.4 Init Skip Diagnostics

The no-init configuration crashes with:

```
Invalid trap address: 0x00000000, cause: LoadAccessFault(0x00000008)
```

Diagnostic test progression:

| Test | Description | Result |
|---|---|---|
| 1 | Init only, return static buffer | PASS (15.8M cycles) |
| 2 | Init + call `risc0_main(10)` | PASS |
| 3 | Init + `risc0_main_eth2(empty)` + access result | CRASH |
| 3b | Same as above, ignore result | PASS |
| 5 | Output return value as raw pointer | **return value = NULL** |
| 8 | Read memory at closed term address | **still NULL after Guest init** |
| 9 | Diagnose initialization steps | **`initialize_Init` fails** |
| 12 | Test Init submodules individually | **`initialize_Init_Data` fails** |
| 13 | Call Data first → Init → Guest | **all succeed** |
| 15 | Workaround + real data | **STF execution succeeds** |

Root cause: `initialize_Init_Data()` fails because its initialization path invokes libc file operations, which the zkVM's `shims.c` stubs return `-1` for. The workaround (calling `initialize_Init_Data()` first to set `_G_initialized = true`, then calling `initialize_Init()` which skips the already-"initialized" Data module) resolves the issue.

## 5. Discussion

### 5.1 Overhead Factors in Direct Execution

Three factors contribute to the Lean cycle overhead relative to Rust:

1. **Fixed Init cost (~15M cycles).** Initialization of 392 modules runs once per execution regardless of input size. At N=10, this accounts for approximately 57% of total cycles (15.8M of 26.1M). The Init cost is amortized as input size grows but remains a significant constant overhead.

2. **Persistent data structure cost.** Lean's `Array.set!` triggers a full array copy when the reference count is not 1. In a stateful computation like the STF — where the `BeaconState` is threaded through multiple transformation functions — reference counts frequently exceed 1 due to struct update syntax (`{ state with ... }`), triggering copies. The cost scales with array size, which is proportional to N.

3. **Reference counting operations.** Every struct update via `{ state with ... }` generates RC increment/decrement instructions. These per-field operations add constant-factor overhead to each state transformation step.

The 17.7x ELF size difference is dominated by the statically linked Init library (~4.0 MB). The Lean runtime adds ~1.0 MB, the guest STF code ~2.5 MB, and libc/libstdc++ ~0.1 MB. The Rust baseline includes only the zkVM runtime at 373 KB.

### 5.2 Scaling Behavior

The Lean/Rust cycle ratio worsens from 2.0x at N=1 to 3.7x at N=1000. This is explained by the interaction between fixed and variable costs:

- At small N, the fixed Init cost dominates both implementations' totals, and Lean's overhead is primarily the ~15M Init cycles — yielding a moderate ratio.
- As N grows, the variable cost (state processing) increases for both implementations, but Lean's variable cost grows faster due to persistent data structure copies and reference counting. The Rust implementation, using mutable in-place updates, scales more efficiently.

The validator count N affects STF cost through three mechanisms:

1. **State decoding/encoding is proportional to N** — variable-length arrays (`validators`, `balances`, participation flags, `inactivityScores`) must be fully serialized and deserialized. Reference: `guest/Guest/Eth2/Serialize.lean`.

2. **Proposer/active validator computation scans all N validators** — `getActiveValidatorIndices` performs a full scan. Reference: `guest/Guest/Eth2/Helpers.lean`.

3. **Epoch processing sub-functions have N-dependent loops** — rewards/penalties, inactivity updates, effective balance updates, and registry updates each iterate over the validator set. References: `guest/Guest/Eth2/Transition/Epoch/{RewardsAndPenalties,InactivityUpdates,EffectiveBalances,RegistryUpdates}.lean`.

The benchmark input (slot 100 → 101) does not cross an epoch boundary, so decode/encode and block-side costs dominate. Even so, the incremental cost from increasing N is clearly visible.

### 5.3 IR Trace Approach Barriers

Three barriers were identified for the IR trace verification approach:

1. **High cost of reproducing IR execution semantics.** Lean IR includes instructions for reference count manipulation (`inc`, `dec`, `del`), memory reuse optimization (`reset`, `reuse`), and control flow (`case`, `jmp`). A verifier must faithfully reproduce RC and heap state transitions for each instruction. The reference implementation (`ir_interpreter.cpp`) is approximately 1,200 lines of C++ with significant semantic complexity.

2. **Host-specific dependencies.** The reference interpreter resolves external function symbols via `dlsym` (POSIX) or `GetProcAddress` (Windows). In the zkVM environment, dynamic linking is unavailable. External function dispatch would need to be pre-resolved and embedded in the trace, adding implementation complexity and trace size.

3. **Trace I/O cost.** In the zkVM, proving cost is proportional to cycle count, and memory access (including reading input data) contributes to that cost. For a full ETH2 STF execution, the IR trace would be large — encompassing every instruction across decode, state transition, and encode phases for potentially thousands of validators. The I/O and paging overhead of loading this trace into the guest may itself be prohibitive.

### 5.4 Comparison of Approaches

| Dimension | Direct Execution (A) | IR Trace Verification (B) |
|---|---|---|
| **Feasibility** | Demonstrated; produces correct results | Not yet demonstrated; qualitative assessment only |
| **Quantitative data** | Cycle counts measured for N=1,10,100,1000 | No data (no prototype built) |
| **Init overhead** | ~15M cycles fixed cost; requires Init_Data workaround | Avoided entirely (Init runs on host) |
| **Variable overhead** | RC operations + persistent data structure copies | IR instruction verification + trace I/O |
| **Implementation complexity** | Moderate: C wrapper, shims, CMake cross-compilation | High: full IR semantics, heap simulation, symbol resolution |
| **Binary size** | 6.6 MB (17.7x Rust) | Unknown; verifier binary + embedded trace |
| **Scalability with N** | 2.0x–3.7x Rust (worsens with N) | Unknown; trace size grows with N |

### 5.5 Open Items

**Direct Execution:**
- Implement cryptographic primitives (`hash_tree_root`, BLS verification)
- RANDAO-based proposer selection
- Test cases crossing epoch boundaries
- Measurements with larger validator sets (N > 1000)
- Investigation of Init fixed cost reduction (excluding unnecessary modules)

**IR Trace Verification:**
- Limited PoC scoped to partial functions (e.g., `processSlots`)
- Minimal opcode set implementation
- Go/No-Go criteria definition (e.g., cycle count and trace size upper bounds)

## References

- [eth2book — State Transition](https://eth2book.info/latest/part3/transition/)
- [ethereum/consensus-specs](https://github.com/ethereum/consensus-specs)
- [Lean IR interpreter implementation](https://github.com/leanprover/lean4/blob/master/src/library/ir_interpreter.cpp)
- [Lean IR instruction definitions](https://github.com/leanprover/lean4/blob/master/src/Lean/Compiler/IR/Basic.lean)
- [RISC Zero Guest Optimization Guide](https://github.com/risc0/risc0/blob/main/website/api/zkvm/optimization.md)
- Source documents:
  - `docs/eth2-stf-direct-execution.md` — Direct execution approach results
  - `docs/eth2-stf-ir-trace-feasibility.md` — IR trace verification feasibility study
  - `docs/lean-vs-rust-zkvm-performance.md` — Sum function benchmarks and Init cost background
