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
```

The `just build` pipeline:
1. `lake build` — compiles Lean to C IR in `guest/.lake/build/ir/`
2. Copies `.c` files to `guest_build/risc0_ir/`
3. CMake cross-compiles to RISC-V → `guest_build/_build/libGuest.a`
4. Copies `libGuest.a` to `methods/guest/lib/`
5. `cargo build --release` — builds Rust host and guest with linking

## Architecture

```
host/src/main.rs          → Rust host: reads input, invokes zkVM prover, outputs receipt
    ↓ (proof generation via risc0-zkvm)
methods/guest/src/main.rs → Rust guest (RISC-V, #![no_main]): reads input, calls Lean via C FFI
    ↓ (extern "C" call)
methods/guest/risc0_lean.c → C wrapper: initializes Lean runtime, calls risc0_main()
    ↓ (C FFI)
guest/Guest.lean          → Lean entry point: @[export risc0_main], delegates to Guest.Basic
guest/Guest/Basic.lean    → Lean business logic (e.g., sum function)
```

**Key linking files:**
- `methods/guest/build.rs` — links libGuest.a, libc, libstdc++, Lean runtime; sets RISC-V linker flags
- `methods/guest/shims.c` — C stubs for syscalls unsupported in zkVM (file I/O, signals, threading, exceptions)
- `guest_build/CMakeLists.txt` — CMake config for cross-compiling Lean C IR to RISC-V
- `guest_build/toolchains/riscv32im-risc0-zkvm-elf.cmake` — RISC-V cross-compiler toolchain file

## Environment Variables

- `LEAN_RISC0_PATH` — Path to Lean RISC0 runtime (default: `$HOME/.lean-risc0`)
- `RISC0_TOOLCHAIN_PATH` — Path to RISC0 toolchain (default: `$HOME/.risc0/toolchains/v2024.1.5-cpp-x86_64-unknown-linux-gnu/riscv32im-linux-x86_64`)

## Toolchain Requirements

- **Lean:** 4.22.0 (managed by Lake)
- **Rust:** stable channel (via rust-toolchain.toml)
- **CMake:** 3.18+
- **just:** command runner
- **RISC0 toolchain:** v2024.1.5 (provides riscv32 cross-compiler, libc, libstdc++)
- **Lean RISC0 runtime:** from [lean-risc0-runtime](https://github.com/anoma/lean-risc0-runtime)

## zkVM Constraints

The guest runs in a restricted RISC-V environment with no OS:
- No file I/O, no signals, no threading, no exception unwinding
- Custom `_sbrk()` with 64MB heap limit (in shims.c)
- Atomic operations are stubbed (only `__atomic_fetch_sub_4` implemented)
- Linker uses `--allow-multiple-definition` for symbol conflicts between runtimes

## Performance Notes

- Full build with Init library initialization: ~13 minutes proving time (~400 modules)
- The `sum-example` branch has a lightweight variant without Init (few seconds)

## Branch Conventions

- `main` — full example with Lean Init library
- `sum-example` — lightweight example without runtime initialization
- `feat/*` — feature branches
