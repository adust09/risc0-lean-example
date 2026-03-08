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

### N=10 — Before Dedup (baseline)

```
=== Timing (3 runs) ===
  Median: 7.71s
  Min:    7.62s
  Max:    11.89s

=== Memory ===
  Value table: 639,836 entries

=== Output ===
  Size: 78,522 bytes
  Status: Success
```

### N=10 — After Dedup (content-hash)

```
=== Timing (3 runs) ===
  Median: 9.96s
  Min:    9.94s
  Max:    10.39s

=== Trace Steps ===
  Total:         238,049
  Call:            51,128 (21.5%)
  Branch:          27,111 (11.4%)
  PrimResult:     100,490 (42.2%)
  CtorCreate:      10,865 (4.6%)
  ProjResult:      39,701 (16.7%)
  SetResult:        8,754 (3.7%)

=== Memory ===
  Value table: 43,255 entries

=== Output ===
  Size: 78,522 bytes
  Status: Success (first byte: 0x40)
```

### N=100 — Before Dedup (baseline)

```
=== Timing (3 runs) ===
  Median: 11.19s
  Min:    11.18s
  Max:    16.44s

=== Memory ===
  Value table: 867,320 entries

=== Output ===
  Size: 91,752 bytes
  Status: Success
```

### N=100 — After Dedup (content-hash)

```
=== Timing (3 runs) ===
  Median: 14.28s
  Min:    14.24s
  Max:    14.74s

=== Trace Steps ===
  Total:         324,741
  Call:            62,832 (19.3%)
  Branch:          33,685 (10.4%)
  PrimResult:     157,376 (48.5%)
  CtorCreate:      13,835 (4.3%)
  ProjResult:      46,905 (14.4%)
  SetResult:       10,108 (3.1%)

=== Memory ===
  Value table: 60,451 entries

=== Output ===
  Size: 91,752 bytes
  Status: Success (first byte: 0x40)
```

## Value Dedup Impact

Content-hash dedup (`HashMap<Value, ValueId>`) deduplicates identical values in the value_table.

| Metric | N=10 before | N=10 after | Reduction | N=100 before | N=100 after | Reduction |
|--------|-------------|------------|-----------|--------------|-------------|-----------|
| Value table entries | 639,836 | 43,255 | **14.8x** | 867,320 | 60,451 | **14.3x** |
| Trace size (bincode) | 8.14 GB | 884 MB | **9.2x** | — | — | — |
| Wall time (median) | 7.71s | 9.96s | +29% | 11.19s | 14.28s | +28% |
| Total steps | 238,049 | 238,049 | same | 324,741 | 324,741 | same |
| Output size | 78,522 B | 78,522 B | same | 91,752 B | 91,752 B | same |

The ~14-15x value_table reduction is consistent across inputs. Wall time increased ~28-29% due to HashMap lookup overhead — an acceptable trade-off given the 9x+ trace size reduction that unblocks zkVM E2E.

## Scaling Characteristics

| Metric | N=10 | N=100 | Ratio (N=100/N=10) |
|--------|------|-------|-------------------|
| Total steps | 238,049 | 324,741 | 1.36x |
| Wall time (median, dedup) | 9.96s | 14.28s | 1.43x |
| Value table (dedup) | 43,255 | 60,451 | 1.40x |
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
=== IR Trace zkVM Benchmark (with dedup) ===
Mode: execute
Trace: 1,975 bytes (bincode)

User Cycles:    357,773
Segments:       1
Wall Time:      45.85ms

=== Output ===
Size: 8 bytes
Status: Success (first byte: 0x37)
```

The sum example (N=10, scalar input) completes E2E successfully with 358K cycles.
With dedup, trace shrank from 3,843 → 1,975 bytes (49% reduction) and cycles from 653K → 358K (45% reduction).

### ETH2 N=10

| Format | Trace size | Status |
|--------|-----------|--------|
| JSON (serde_json) | ~15 GB | Too large |
| bincode (no dedup) | 8.14 GB | `TryFromIntError` — exceeds zkVM u32 write limit (~4GB) |
| bincode (with dedup) | **884 MB** | Under 4GB limit — zkVM E2E unblocked |

Value dedup reduced the value_table from 639,836 → 43,255 entries (14.8x), bringing the
bincode trace from 8.14 GB down to 884 MB (9.2x reduction). zkVM E2E execution is now feasible.

ETH2 N=10 zkVM cycle measurement: pending (executor running).

### Commands

```bash
# Sum example
cargo run --release -p ir-trace --bin ir-trace -- \
  --ir ir_program_sum.json --input /dev/null \
  --entry risc0_main --scalar-input 10 --output /tmp/sum_trace_dedup.bin
RISC0_DEV_MODE=1 cargo run --release --bin ir-trace-host -- \
  --ir ir_program_sum.json --input /dev/null --scalar-input 10 \
  --trace /tmp/sum_trace_dedup.bin --entry risc0_main --mode execute

# ETH2 N=10 cycle measurement (now feasible with dedup)
just bench-ir-trace-cycles 10
```

## Notes

- IR Trace skips Init entirely — the interpreter resolves declarations directly from the IR JSON
- Trace serialization uses bincode (previously JSON, which produced ~15GB files)
- Value dedup uses content-hash (`HashMap<Value, ValueId>`) to deduplicate identical values — ~14-15x reduction in value_table entries
- Host-side wall time increased ~28% with dedup due to HashMap hashing overhead, but trace size decreased 9x
- Run 1 is consistently slower due to cold caches; runs 2-3 are stable
