# IR Trace Benchmark Results

## Overview

IR Trace is the third approach to running Lean ETH2 STF in zkVM, alongside compiled Lean and compiled Rust.
Instead of compiling Lean to C/RISC-V, it interprets the Lean lambda-RC IR on the host and generates an execution trace for zkVM verification.

## 3-Approach Comparison

| Approach | N=10 cycles | N=10 seg | N=100 cycles | N=100 seg | Notes |
|----------|-------------|----------|--------------|-----------|-------|
| Lean (compiled, init) | 26,148,291 | 29 | 35,281,299 | 38 | Includes Init (~15M cycles) |
| Rust (compiled) | 12,491,509 | 13 | 14,446,747 | 15 | Baseline |
| IR Trace (zkVM verify) | TBD | TBD | TBD | TBD | bincode trace, execute mode |
| IR Trace (host interp) | 238,049 steps / 7.71s | - | 324,741 steps / 11.19s | - | Host-side only |

## IR Trace Detailed Results

### N=10 (3 runs, median)

```
=== Timing (3 runs) ===
  Median: 7.71s
  Min:    7.62s
  Max:    11.89s

=== Trace Steps ===
  Total:         238,049
  Call:            51,128 (21.5%)
  Branch:          27,111 (11.4%)
  PrimResult:     100,490 (42.2%)
  CtorCreate:      10,865 (4.6%)
  ProjResult:      39,701 (16.7%)
  SetResult:        8,754 (3.7%)

=== Memory ===
  Value table: 639,836 entries

=== Output ===
  Size: 78,522 bytes
  Status: Success
```

### N=100 (3 runs, median)

```
=== Timing (3 runs) ===
  Median: 11.19s
  Min:    11.18s
  Max:    16.44s

=== Trace Steps ===
  Total:         324,741
  Call:            62,832 (19.3%)
  Branch:          33,685 (10.4%)
  PrimResult:     157,376 (48.5%)
  CtorCreate:      13,835 (4.3%)
  ProjResult:      46,905 (14.4%)
  SetResult:       10,108 (3.1%)

=== Memory ===
  Value table: 867,320 entries

=== Output ===
  Size: 91,752 bytes
  Status: Success
```

## Scaling Characteristics

| Metric | N=10 | N=100 | Ratio (N=100/N=10) |
|--------|------|-------|-------------------|
| Total steps | 238,049 | 324,741 | 1.36x |
| Wall time (median) | 7.71s | 11.19s | 1.45x |
| Value table | 639,836 | 867,320 | 1.36x |
| PrimResult steps | 100,490 | 157,376 | 1.57x |
| Output size | 78,522 B | 91,752 B | 1.17x |

PrimResult (arithmetic ops) scales most steeply with validator count, as expected from per-validator balance/epoch calculations.

## Output Consistency

| Approach | N=10 output | N=100 output |
|----------|-------------|--------------|
| Lean (compiled, init) | 78,746 B | 91,976 B |
| Rust (compiled) | 78,746 B | 91,976 B |
| IR Trace | 78,522 B (-224 B) | 91,752 B (-224 B) |

IR Trace outputs are 224 bytes smaller than Lean/Rust across both inputs. This is likely due to a difference in `gen_eth2_input` serialization format vs the host-side input used for the compiled guests.

## Measurement Commands

```bash
# Single run benchmark
cargo run --release -p ir-trace --bin ir-trace -- \
  --ir ir_program_filtered.json \
  --input /tmp/eth2_input_10.bin \
  --entry risc0_main_eth2 --bench --runs 3

# Full scaling benchmark (N=10,100)
just bench-ir-trace 10,100
```

## zkVM Cycle Measurement

Trace serialization was switched from JSON to bincode to reduce trace size.

### Sum Example (E2E verified)

```
=== IR Trace zkVM Benchmark ===
Mode: execute
Trace: 3,843 bytes (bincode)

User Cycles:    653,173
Segments:       1
Wall Time:      76.18ms

=== Output ===
Size: 8 bytes
Status: Success (first byte: 0x37)
```

The sum example (N=10, scalar input) completes E2E successfully with 653K cycles.

### ETH2 N=10 (blocked)

| Format | Trace size | Status |
|--------|-----------|--------|
| JSON (serde_json) | ~15 GB | Too large |
| bincode | 8.14 GB | `TryFromIntError` — exceeds zkVM u32 write limit (~4GB) |

Root cause: the value_table contains 639,837 entries including large ByteArray and Object values.
The bincode compression ratio was only ~1.8x (not the expected 5-10x), because the dominant cost is
serializing complex nested Value structures, not JSON overhead.

See [problem.md](./problem.md) for detailed analysis and proposed solutions.

### Commands

```bash
# Sum example (works)
cargo run --release -p ir-trace --bin ir-trace -- \
  --ir ir_program.json --input /dev/null \
  --entry risc0_main --scalar-input 10 --output /tmp/sum_trace.bin
RISC0_DEV_MODE=1 cargo run --release --bin ir-trace-host -- \
  --ir ir_program.json --input /dev/null --scalar-input 10 \
  --trace /tmp/sum_trace.bin --entry risc0_main --mode execute

# ETH2 cycle measurement (currently blocked by trace size)
just bench-ir-trace-cycles 10
```

## Notes

- IR Trace skips Init entirely — the interpreter resolves declarations directly from the IR JSON
- Trace serialization uses bincode (previously JSON, which produced ~15GB files)
- Host-side wall-clock time is measured (not directly comparable to zkVM cycles, but shows interpreter overhead)
- Run 1 is consistently slower (11.9s / 16.4s) due to cold caches; runs 2-3 are stable (~7.6s / ~11.2s)
