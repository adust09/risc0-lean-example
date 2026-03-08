# IR Trace: Trace Size Problem

## Problem

The IR Trace approach generates an execution trace on the host and passes it to the zkVM guest for verification.
For the ETH2 STF with N=10 validators, the trace is **8.14 GB** (bincode), which exceeds the zkVM's ~4 GB input limit (`Vec<u8>` serialized via `env::write()` uses u32 length prefix).

The sum example (scalar input, 64 steps) works fine at 3.8 KB. The problem is specific to complex workloads with large intermediate values.

## Why the Trace Is So Large

| Component | N=10 | Notes |
|-----------|------|-------|
| Steps | 238,049 | Modest — each step is small |
| Value table entries | 639,837 | **This is the bottleneck** |
| Trace size (bincode) | 8.14 GB | ~12.7 KB per value on average |
| Trace size (JSON) | ~15 GB | 1.8x larger than bincode |

The value_table stores every intermediate `Value` created during interpretation:
- `ByteArray` values (ETH2 state is ~78 KB, copied/sliced many times)
- Nested `Object` values with recursive fields
- `Array` values containing other Values

Many values are near-duplicates (e.g., a ByteArray modified at one index produces a full copy).

## Why Compiled Lean Doesn't Have This Problem

| | Compiled Lean | IR Trace |
|---|---|---|
| Input to zkVM guest | ETH2 state+block (~79 KB) | ETH2 input + **entire trace** (8 GB) |
| Computation | Runs natively inside zkVM | Host interprets, guest verifies |
| Intermediate values | In zkVM memory (paged) | Serialized in trace file |

Compiled Lean runs the actual computation inside the zkVM, so intermediate values live in zkVM memory and are never serialized. IR Trace externalizes all computation to the host and must pass every intermediate result to the guest.

## Proposed Solutions

### 1. Value Deduplication + Hash References

Many values in the table are duplicates or near-duplicates. Replace repeated values with references:

```
Before: value_table = [ByteArray([0,1,2,...78KB]), ByteArray([0,1,2,...78KB]), ...]
After:  value_table = [ByteArray([0,1,2,...78KB]), Ref(0), ...]
```

Expected reduction: 2-5x for ETH2 workloads (heavy ByteArray reuse).

### 2. Delta Encoding for ByteArrays

ETH2 state transitions modify small portions of large byte arrays. Store only the diff:

```
value_table[N] = Delta { base: value_id_M, offset: 1234, patch: [0x42] }
```

Expected reduction: 10-50x for ByteArray-heavy workloads.

### 3. Streaming / Chunked Verification

Split the trace into chunks that each fit in zkVM memory. Verify each chunk independently with hash chaining:

```
Chunk 1: steps[0..1000]     → commitment_1
Chunk 2: steps[1000..2000]  → commitment_2 (depends on commitment_1)
...
Final: combine all commitments
```

This removes the single-input size limit entirely but requires architectural changes to the verifier.

### 4. Prune Unused Values

Not all values in the value_table are needed for verification. Dead values (not referenced by any step or output) can be removed. Requires a liveness analysis pass after trace generation.

## Recommended Next Step

**Option 1 (quickest)**: Value deduplication — add content-hash based dedup to `build_trace()` in `tools/ir-trace/src/trace_format.rs`. This is a localized change that doesn't affect the verifier logic.

**Option 2 (most impactful)**: Streaming verification — eliminates the size limit entirely but requires redesigning the guest verifier and host driver.

## Current Status

- Sum example: E2E works (653K cycles, 3.8 KB trace)
- ETH2 N=10: Trace generates successfully but cannot be passed to zkVM (8.14 GB > 4 GB limit)
- ETH2 N=100: Not attempted (trace would be even larger)
