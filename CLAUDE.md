# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Cross-compilation toolchain that runs Lean 4 code inside RISC Zero's zkVM (zero-knowledge virtual machine). The pipeline: **Lean 4 → C (via Lake) → RISC-V 32-bit objects (via CMake) → linked into Rust guest → proven by Rust host**.

## Build Commands

```bash
# Full build pipeline (Lean → C → RISC-V → Rust)
just build

# Clean all artifacts
just clean

# Build only the Lean project
cd guest && lake build

# Build only the C→RISC-V step
cd guest_build && just build

# Build only the Rust host/guest
cargo build --release

# Run the example (computes sum(N))
target/release/host <N>

# Fast dev iteration (skips actual proving, uses fake receipts)
RISC0_DEV_MODE=1 cargo run --release --bin host -- <N>
```

### Benchmark Commands

```bash
# Run execution benchmarks (Lean vs Rust guest)
just bench-execute

# Run proving benchmarks
just bench-prove

# Profile a specific guest with custom input
just bench-profile-lean N=1000
just bench-profile-rust N=1000
```

The benchmark binary (`host/src/bin/benchmark.rs`) supports `--mode execute|prove`, `--guest lean|rust|both`, `--inputs 10,100,1000`, and `--runs 3`.

### Build Pipeline Detail

`just build` runs these steps sequentially:
1. `lake build` — compiles Lean to C IR in `guest/.lake/build/ir/`
2. Copies `.c` files to `guest_build/risc0_ir/`
3. CMake cross-compiles to RISC-V → `guest_build/_build/libGuest.a`
4. Copies `libGuest.a` to `methods/guest/lib/`
5. `cargo build --release` — builds Rust host and both guest ELFs with linking

## Architecture

### Execution Flow (Lean Guest)

```
host/src/main.rs          → Rust host: reads input, invokes zkVM prover, outputs receipt
    ↓ (proof generation via risc0-zkvm)
methods/guest/src/main.rs → Rust guest (#![no_main]): reads input, calls lean_simple_risc0_main() via FFI
    ↓ (extern "C" call)
methods/guest/risc0_lean.c → C wrapper: forwards to risc0_main() (the Lean-exported function)
    ↓ (C FFI)
guest/Guest.lean          → Lean entry point: @[export risc0_main], delegates to Guest.Basic
guest/Guest/Basic.lean    → Lean business logic (e.g., sum function using UInt32)
```

### Dual-Guest Setup

Two guest programs exist for benchmarking comparison:
- **`methods/guest/`** — Lean guest: Rust → C FFI → Lean compiled code, links `libGuest.a`, Lean runtime, libc, libstdc++
- **`methods/guest-rust/`** — Pure Rust guest: identical algorithm implemented natively in Rust (no FFI, no C)

Both are compiled as separate RISC-V ELFs via `risc0-build`. The `methods/src/lib.rs` auto-generates constants: `METHOD_ELF`/`METHOD_ID` (Lean) and `GUEST_RUST_ELF`/`GUEST_RUST_ID` (Rust).

### Key Linking Files

- `methods/guest/build.rs` — Links libGuest.a, libc, libstdc++, Lean runtime; sets `--allow-multiple-definition` linker flag
- `methods/guest/shims.c` — C stubs for syscalls unsupported in zkVM (file I/O, signals, threading, exceptions, `_sbrk` with 64MB heap)
- `methods/guest/risc0_lean.c` — Thin C wrapper: `lean_simple_risc0_main()` → `risc0_main()` (Lean-exported)
- `guest_build/CMakeLists.txt` — CMake config for cross-compiling Lean C IR to RISC-V
- `guest_build/toolchains/riscv32im-risc0-zkvm-elf.cmake` — RISC-V cross-compiler toolchain file

## Environment Variables

- `LEAN_RISC0_PATH` — Path to Lean RISC0 runtime (default: `$HOME/.lean-risc0`)
- `RISC0_TOOLCHAIN_PATH` — Path to RISC0 toolchain (default: `$HOME/.risc0/toolchains/v2024.1.5-cpp-x86_64-unknown-linux-gnu/riscv32im-linux-x86_64`)
- `RISC0_DEV_MODE=1` — Skip actual proving for fast iteration during development
- `RISC0_PPROF_OUT=profile.pb` — Output pprof profiling data

## Toolchain Requirements

- **Lean:** 4.22.0 (pinned in `guest/lean-toolchain`, managed by Lake)
- **Rust:** stable channel (via `rust-toolchain.toml`, includes `rust-src` for guest compilation)
- **CMake:** 3.18+
- **just:** command runner
- **RISC0 toolchain:** v2024.1.5 (provides riscv32 cross-compiler, libc, libstdc++)
- **Lean RISC0 runtime:** from [lean-risc0-runtime](https://github.com/anoma/lean-risc0-runtime)
- **Lean RISC0 Init library:** from [lean-risc0-init](https://github.com/anoma/lean-risc0-init)

## zkVM Constraints

The guest runs in a restricted RISC-V environment with no OS:
- No file I/O, no signals, no threading, no exception unwinding
- Custom `_sbrk()` with 64MB heap limit (in `shims.c`)
- Atomic operations are stubbed (only `__atomic_fetch_sub_4` implemented)
- Linker uses `--allow-multiple-definition` for symbol conflicts between runtimes

## Modifying Business Logic

To change what the Lean guest computes:
1. Edit `guest/Guest/Basic.lean` with new logic
2. Update `guest/Guest.lean` — the `@[export risc0_main]` function is the C FFI entry point
3. If changing the FFI signature, also update `methods/guest/risc0_lean.c` and `methods/guest/src/main.rs`
4. If benchmarking, mirror the algorithm in `methods/guest-rust/src/main.rs`
5. Run `just build` to rebuild the full pipeline

## Branch Conventions

- `main` — full example with Lean Init library
- `sum-example` — lightweight example without runtime initialization
- `feat/*` — feature branches
