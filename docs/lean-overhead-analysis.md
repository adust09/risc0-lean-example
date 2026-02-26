# Lean Guest Overhead Analysis

Analysis of performance overhead when running Lean 4 code inside RISC Zero zkVM,
compared to an equivalent pure Rust implementation.

## Benchmark Summary

**Test function:** `sum(n) = if n == 0 then 0 else (n + sum(n-1)) &&& 0xFFFF`

Both guests compute the identical algorithm. The Lean guest goes through the
full Lean → C → RISC-V pipeline, while the Rust guest is a direct implementation.

### Execution Mode Results (no proving)

| Guest | N | User Cycles | Segments | Wall Time |
|-------|------:|------------:|---------:|----------:|
| Lean  |    10 |   4,119,771 |        5 |    12.3s  |
| Rust  |    10 |       3,736 |        1 |     41ms  |
| **Ratio** | | **1,102.7x** | **5.0x** | **~300x** |
| | | | | |
| Lean  |   100 |   4,124,252 |        5 |    14.6s  |
| Rust  |   100 |       5,176 |        1 |     39ms  |
| **Ratio** | | **796.8x** | **5.0x** | **~375x** |
| | | | | |
| Lean  | 1,000 |   4,161,410 |        6 |    10.4s  |
| Rust  | 1,000 |      19,576 |        1 |     74ms  |
| **Ratio** | | **212.6x** | **6.0x** | **~140x** |

### ELF Binary Size

| | Lean | Rust | Ratio |
|--|-----:|-----:|------:|
| ELF size | 5,307,860 bytes (5.1 MB) | 276,472 bytes (270 KB) | **19.2x** |

## Overhead: 4-Layer Structure

### Layer 1: Init Library Initialization (~99% of cycles)

This is the dominant source of overhead. The evidence is clear: user cycles are
nearly constant regardless of input size.

```
N=10:    4,119,771 cycles
N=100:   4,124,252 cycles  (delta from N=10:   +4,481 = actual computation)
N=1000:  4,161,410 cycles  (delta from N=10:  +41,639 = actual computation)
```

The baseline ~4,119,000 cycles are entirely spent in `initialize_Init()` and
`lean_initialize_runtime_module()`.

**Call chain** (`methods/guest/risc0_lean.c` → `guest_build/risc0_ir/Guest.c`):

```c
// risc0_lean.c:lean_risc0_main()
lean_initialize_runtime_module();                    // Lean runtime bootstrap
lean_object* res = initialize_Guest(1, lean_io_mk_world());

// Guest.c:initialize_Guest()
res = initialize_Init(builtin, lean_io_mk_world()); // ← ~99% of all cycles
res = initialize_Guest_Basic(builtin, lean_io_mk_world());
```

**Mechanism:** Lean's `import` is transitive. `Guest.Basic` imports `Init`, which
triggers recursive initialization of all 392 modules in the Init library. Each
module allocates static variables (string literals, lookup tables, type info) on
the heap via `lean_mark_persistent()`.

**Library breakdown:**

| Library | Size | Object Files | Role |
|---------|-----:|-------------:|------|
| `libInit.a` | 23 MB | 394 | Init library (392 `initialize_*` functions) |
| `libLean.a` | 944 KB | 44 | Runtime (GC, refcount, memory management) |
| `libGuest.a` | 9.6 KB | 2 | User code (`Guest.c` + `Guest/Basic.c`) |

The user code is only 9.6 KB, but it pulls in 23 MB of initialization code that
must execute before any business logic runs.

### Layer 2: Nat Heap Allocation + Reference Counting

Lean's `Nat` type is a heap-allocated boxed object. Every arithmetic operation
allocates a new `lean_object` and requires reference counting.

**Lean C IR** (`guest_build/risc0_ir/Guest/Basic.c`):

```c
LEAN_EXPORT lean_object* l_sum(lean_object* x_1) {
    x_2 = lean_unsigned_to_nat(0u);     // convert 0 to Nat object
    x_3 = lean_nat_dec_eq(x_1, x_2);    // compare
    if (x_3 == 0) {
        x_4 = lean_unsigned_to_nat(1u);  // convert 1 to Nat object
        x_5 = lean_nat_sub(x_1, x_4);   // heap alloc: n - 1
        x_6 = l_sum(x_5);               // recurse
        lean_dec(x_5);                   // refcount decrement
        x_7 = lean_nat_add(x_1, x_6);   // heap alloc: n + sum(n-1)
        lean_dec(x_6);                   // refcount decrement
        x_8 = lean_unsigned_to_nat(65535u);
        x_9 = lean_nat_land(x_7, x_8);  // heap alloc: result & 0xFFFF
        lean_dec(x_7);                   // refcount decrement
        return x_9;
    }
    return x_2;
}
```

**Equivalent Rust** (`methods/guest-rust/src/main.rs`):

```rust
fn sum(n: u32) -> u32 {
    if n == 0 { 0 } else { (n + sum(n - 1)) & 0xFFFF }
}
```

**Per-recursion overhead:**

| | Lean | Rust |
|--|------|------|
| Heap allocations | 3 (`lean_nat_sub`, `lean_nat_add`, `lean_nat_land`) | 0 |
| Refcount operations | 3 (`lean_dec`) | 0 |
| Type conversions | 3 (`lean_unsigned_to_nat`) | 0 |
| Instructions | ~50 cycles/recursion | ~16 cycles/recursion |

**Derivation:** (4,124,252 - 4,119,771) / 90 iterations = ~50 cycles per recursion
for Lean. (5,176 - 3,736) / 90 = ~16 cycles per recursion for Rust.

### Layer 3: Data Marshalling (FFI Boundary)

The Rust guest cannot pass a `u32` directly to Lean. Data traverses a 6-step
conversion pipeline in each direction:

```
Input:  u32 → String → bytes → [C FFI] → ByteArray → String → Nat
                                                                 ↓
                                                              sum(n)
                                                                 ↓
Output: u32 ← String ← bytes ← [C FFI] ← ByteArray ← String ← Nat
```

**Detail of each step:**

```
Rust main()
  │  env::read::<u32>()           read u32 from zkVM input
  │  input.to_string().into_bytes()   u32 → String → Vec<u8>
  │
  ├─► C: lean_risc0_main()
  │     byte_array_from_c()       byte-by-byte loop copy → Lean ByteArray
  │
  │   ├─► Lean: risc0_main()
  │   │     String.fromUTF8!      ByteArray → String (UTF-8 validation)
  │   │     String.toNat!         String → Nat (decimal parsing)
  │   │     sum n                 actual computation
  │   │     toString result       Nat → String (decimal formatting)
  │   │     String.toUTF8         String → ByteArray
  │   │
  │     c_from_byte_array()       byte-by-byte loop copy + malloc → char*
  │
  │  String::from_utf8()          bytes → String
  │  .parse::<u32>()              String → u32
  │  env::commit(&value)          write u32 to zkVM journal
```

The Rust guest reads and writes `u32` directly via `env::read()`/`env::commit()`,
eliminating all marshalling.

### Layer 4: ELF Size → Paging Cost

In RISC Zero zkVM, loading memory pages has a direct cycle cost (`paging_cycles`
in prove mode). The 19.2x larger ELF means significantly more pages to load.

**ELF composition (Lean guest):**

| Component | Source Size | Notes |
|-----------|------------|-------|
| `libInit.a` | 23 MB | Linker strips unused symbols, but initialization code remains |
| `libLean.a` | 944 KB | Runtime: GC, refcounting, memory management |
| libc + libstdc++ | varies | Required by Lean runtime |
| `libGuest.a` | 9.6 KB | Actual user code |
| Rust zkVM runtime | ~270 KB | Shared with Rust guest |
| **Final ELF** | **5.1 MB** | After linker dead-code elimination |

The Rust guest ELF (270 KB) consists almost entirely of the zkVM runtime itself,
with the `sum` function adding negligible size.

## Quantitative Summary

| Factor | Est. Cycles | Contribution | Evidence |
|--------|------------|-------------|----------|
| Init initialization (392 modules) | ~4,100,000 | **~99%** | Constant baseline across all N |
| Nat heap/RC overhead | ~50/recursion | N-dependent | Delta between N=10 and N=100 |
| Data marshalling | ~hundreds | <0.1% | String conversion only |
| ELF paging | materializes in prove mode | indirect in execute | 19.2x size difference |

## Potential Improvements

These are identified directions, not implementation plans:

1. **Eliminate Init import** — Avoid `import Init` and provide only the required
   definitions manually. The `sum-example` branch demonstrates this approach and
   reduces proving time from ~13 minutes to seconds.

2. **Use UInt32 instead of Nat** — Replace `Nat` with `UInt32` in `Guest/Basic.lean`
   to eliminate heap allocation. `UInt32` compiles to unboxed machine integers.

3. **Simplify marshalling** — Pass `u32` directly via binary encoding instead of
   going through String conversion. This would eliminate the 12-step round-trip.

4. **Lazy/partial Init initialization** — Would require changes to the Lean compiler
   to support on-demand module initialization. Not feasible with current tooling.

## Reproduction

```bash
# Build both guests
just build

# Run execution benchmark
just bench-execute

# Run prove benchmark (much slower)
just bench-prove
```

## References

| File | Description |
|------|-------------|
| `guest/Guest/Basic.lean` | Lean business logic (`sum` function) |
| `guest/Guest.lean` | Lean entry point (`@[export risc0_main]`) |
| `guest_build/risc0_ir/Guest.c` | Compiled C IR: `initialize_Guest`, `initialize_Init` call |
| `guest_build/risc0_ir/Guest/Basic.c` | Compiled C IR: `l_sum` with Nat heap operations |
| `methods/guest/risc0_lean.c` | C wrapper: runtime init + data marshalling |
| `methods/guest/src/main.rs` | Rust guest (Lean): FFI call to `lean_risc0_main` |
| `methods/guest-rust/src/main.rs` | Rust guest (pure): direct `sum` implementation |
| `methods/guest/shims.c` | zkVM shims: `_sbrk` with 64 MB heap |
| `methods/guest/build.rs` | Linker config: libInit.a, libLean.a, libGuest.a |
| `host/src/bin/benchmark.rs` | Benchmark harness: cycle measurement + comparison |
