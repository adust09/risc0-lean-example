# Lean 4 Execution Trace Verification: Research Report

## 1. Background and Motivation

### Current Architecture Problem

The current risc0-lean-example runs the **entire Lean runtime inside the zkVM**:

```
Lean 4 → C IR → RISC-V cross-compile → full execution inside zkVM
```

**Overhead:**

| Metric | Lean Guest | Rust Guest | Ratio |
|--------|-----------|-----------|-------|
| ELF size (sum) | 1.5 MB | 270 KB | 5.6x |
| ELF size (ETH2) | 6.6 MB | 373 KB | 17.7x |
| Init overhead | 4.1M cycles | 0 | - |
| ETH2 N=100 | 35.3M cycles / 38 seg | 14.4M cycles / 15 seg | 2.4x |

What should NOT be inside the zkVM:
- Lean runtime (reference counting, memory management)
- libc, libstdc++
- Init library (392 module initializations)
- OS stubs (shims.c: 53 lines of syscall stubs)

### Proposed Architecture

Following the zkVM philosophy of "verify execution traces instead of re-executing the entire computation":

```
Host (outside zkVM)                        Guest (inside zkVM)
┌────────────────────────────┐            ┌───────────────────────┐
│ Lean IR Interpreter         │            │ Lightweight Trace     │
│ (modified or Rust reimpl)   │  trace     │ Verifier              │
│                            │ ────────→  │ (Rust, pure no_std)   │
│ 1. Load IR declarations    │            │ 1. Read trace         │
│ 2. Evaluate step by step   │            │ 2. Verify each step   │
│ 3. Output trace            │            │ 3. Commit result      │
└────────────────────────────┘            └───────────────────────┘
```

---

## 2. Lean 4 IR (Intermediate Representation) Specification

### 2.1 Compilation Pipeline

```
Lean Source
  ↓ (elaboration & type checking)
Expression Trees
  ↓ (proof term erasure)
LCNF (Lambda Calculus Normal Form, A-normal form)
  ↓ (optimizations: inlining, lambda lifting, join points)
LCNF with RC
  ↓ (reference counting insertion)
IR (Intermediate Representation)
  ↓ (EmitC)
C code → native binary
```

### 2.2 IR Type Definitions (Complete)

Source: `src/Lean/Compiler/IR/Basic.lean`

#### IRType — Type System

```lean
inductive IRType where
  | float                                              -- double
  | uint8 | uint16 | uint32 | uint64 | usize          -- fixed-width integers
  | erased                                             -- type-erased
  | object                                             -- boxed object
  | tobject                                            -- tagged/object union
  | float32                                            -- float
  | struct (leanTypeName : Option Name) (types : Array IRType)  -- struct
  | union (leanTypeName : Name) (types : Array IRType)          -- union
  | tagged                                             -- tagged pointer
  | void                                               -- void
```

#### Expr — Expression Nodes

```lean
inductive Expr where
  | ctor (i : CtorInfo) (ys : Array Arg)               -- constructor application
  | reset (n : Nat) (x : VarId)                        -- memory reset (reuse preparation)
  | reuse (x : VarId) (i : CtorInfo) (updtHeader : Bool) (ys : Array Arg)  -- memory reuse
  | proj (i : Nat) (x : VarId)                         -- boxed field access
  | uproj (i : Nat) (x : VarId)                        -- USize field access
  | sproj (n : Nat) (offset : Nat) (x : VarId)         -- unboxed field access
  | fap (c : FunId) (ys : Array Arg)                   -- full application (all args present)
  | pap (c : FunId) (ys : Array Arg)                   -- partial application (closure creation)
  | ap (x : VarId) (ys : Array Arg)                    -- closure application
  | box (ty : IRType) (x : VarId)                      -- box an unboxed value
  | unbox (x : VarId)                                  -- unbox from object
  | lit (v : LitVal)                                   -- literal
  | isShared (x : VarId)                               -- check if refcount > 1
```

#### FnBody — Function Body (Control Flow)

```lean
inductive FnBody where
  | vdecl (x : VarId) (ty : IRType) (e : Expr) (b : FnBody)       -- variable declaration + evaluation
  | jdecl (j : JoinPointId) (xs : Array Param) (v : FnBody) (b : FnBody)  -- join point declaration
  | set (x : VarId) (i : Nat) (y : Arg) (b : FnBody)              -- boxed field mutation
  | setTag (x : VarId) (cidx : Nat) (b : FnBody)                  -- constructor tag mutation
  | uset (x : VarId) (i : Nat) (y : VarId) (b : FnBody)           -- USize field mutation
  | sset (x : VarId) (i : Nat) (offset : Nat) (y : VarId) (ty : IRType) (b : FnBody)  -- scalar field mutation
  | inc (x : VarId) (n : Nat) (c : Bool) (persistent : Bool) (b : FnBody)  -- refcount increment
  | dec (x : VarId) (n : Nat) (c : Bool) (persistent : Bool) (b : FnBody)  -- refcount decrement
  | del (x : VarId) (b : FnBody)                                  -- object deallocation
  | case (tid : Name) (x : VarId) (xType : IRType) (cs : Array Alt)  -- pattern match branch
  | ret (x : Arg)                                                  -- return value
  | jmp (j : JoinPointId) (ys : Array Arg)                        -- jump to join point
  | unreachable                                                    -- unreachable
```

#### Auxiliary Types

```lean
-- Variable ID
structure VarId where idx : Index

-- Join Point ID
structure JoinPointId where idx : Index

-- Constructor Info
structure CtorInfo where
  name : Name        -- constructor name
  cidx : Nat         -- tag index
  size : Nat         -- boxed field count
  usize : Nat        -- USize field count
  ssize : Nat        -- scalar field byte size

-- Parameter
structure Param where
  x : VarId
  borrow : Bool      -- borrow semantics
  ty : IRType

-- Argument
inductive Arg where
  | var (id : VarId)
  | erased

-- Literal Value
inductive LitVal where
  | num (v : Nat)
  | str (v : String)

-- Pattern Match Alternative
inductive Alt where
  | ctor (info : CtorInfo) (b : FnBody)
  | default (b : FnBody)

-- Top-level Declaration
inductive Decl where
  | fdecl (f : FunId) (xs : Array Param) (type : IRType) (body : FnBody) (info : DeclInfo)
  | extern (f : FunId) (xs : Array Param) (type : IRType) (ext : ExternAttrData)
```

### 2.3 IR Representation Example: sum(UInt32)

Lean source:
```lean
partial def sum (n : UInt32) : UInt32 :=
  if n == 0 then 0 else (n + sum (n - 1)) &&& 0xFFFF
```

Estimated IR (reverse-engineered from C output):
```
fdecl l_sum (x_1 : uint32) : uint32 :=
  let x_2 : uint32 := lit 0
  let x_3 : uint8  := fap lean_uint32_dec_eq [x_1, x_2]   -- n == 0
  case x_3 : uint8 of
    | 0 =>                                                   -- false branch
      let x_4 : uint32 := lit 1
      let x_5 : uint32 := fap lean_uint32_sub [x_1, x_4]   -- n - 1
      let x_6 : uint32 := fap l_sum [x_5]                   -- sum(n-1)
      let x_7 : uint32 := fap lean_uint32_add [x_1, x_6]   -- n + sum(n-1)
      let x_8 : uint32 := lit 65535
      let x_9 : uint32 := fap lean_uint32_land [x_7, x_8]  -- & 0xFFFF
      ret x_9
    | default =>                                             -- true branch
      let x_10 : uint32 := lit 0
      ret x_10
```

Characteristics:
- **UInt32 is an unboxed type** → `ctor`, `proj`, `box/unbox`, `inc/dec` are unnecessary
- **Everything is `vdecl` + `fap`** → very simple instruction sequence
- **Primitive operations are `fap` external function calls** → `lean_uint32_add`, etc.
- **Branching uses `case` to check Bool tag value (0=false, 1=true)**

---

## 3. ir_interpreter.cpp Structure

Source: `src/library/ir_interpreter.cpp`

### 3.1 Main Data Structures

```cpp
// Union value representation
union value {
    uint64   m_num;     // unboxed integer
    double   m_float;
    float    m_float32;
    object * m_obj;     // boxed object
};

// Execution frame
struct frame {
    name   m_fn;        // function name
    size_t m_arg_bp;    // argument stack base pointer
    size_t m_jp_bp;     // join point stack base pointer
};

// Interpreter core
class interpreter {
    std::vector<value>            m_arg_stack;      // variable slots
    std::vector<fn_body const *>  m_jp_stack;       // join points
    std::vector<frame>            m_call_stack;     // call stack
    name_hash_map<constant_cache_entry>  m_constant_cache;  // constant cache
    name_hash_map<symbol_cache_entry>    m_symbol_cache;    // symbol cache
};
```

### 3.2 Execution Flow

#### Entry Point

```
run_boxed(env, opts, fn_name, args)
  → call_boxed(fn_name, n, args)
    → call(fn_name, args)     ← native or interpreted
      → eval_body(fn_body)    ← main loop
```

#### eval_body() — Main Loop

Processes each `FnBody` node sequentially:

| FnBody | Processing |
|--------|-----------|
| `vdecl(x, ty, e, b)` | Evaluate expression with `eval_expr(e)`, store result in `var(x)`, proceed to `b`. **Tail-call optimization present** |
| `jdecl(j, xs, v, b)` | Save join point `v` in `m_jp_stack[j]`, proceed to `b` |
| `case(tid, x, ty, cs)` | Get tag value of `var(x)`, proceed to matching `Alt`'s `FnBody` |
| `ret(x)` | Return `var(x)` (function exit) |
| `jmp(j, ys)` | Evaluate args `ys`, proceed to join point `j`'s body |
| `inc(x, n, ...)` | Increment refcount of `var(x)` by `n` |
| `dec(x, n, ...)` | Decrement refcount by `n` |
| `del(x)` | Deallocate `var(x)` |
| `set(x, i, y)` | Set field `i` of `var(x)` to `var(y)` |
| `unreachable` | Panic |

#### eval_expr() — Expression Evaluation

| Expr | Processing |
|------|-----------|
| `ctor(info, ys)` | Allocate object with `alloc_ctor()`, set fields from arguments |
| `proj(i, x)` | Read field `i` of `var(x)` |
| `fap(fn, ys)` | Full application: `call(fn, ys)` |
| `pap(fn, ys)` | Partial application: closure creation |
| `ap(x, ys)` | Closure application: `apply_n()` |
| `box(ty, x)` | `box_t(var(x), ty)` |
| `unbox(x)` | `unbox_t(var(x).m_obj, ty)` |
| `lit(v)` | Return literal value |
| `isShared(x)` | `!is_exclusive(var(x).m_obj)` |

#### call() — Function Call

```
call(fn_name, args)
  1. lookup_symbol(fn_name) for symbol resolution
  2. If native code exists:
     a. Box arguments (type-dependent)
     b. Call native function via curry()
     c. Unbox return value
  3. If IR declaration exists:
     a. Push arguments to stack
     b. Push frame
     c. eval_body(decl.body)
     d. Pop and return result
```

### 3.3 Existing Trace Infrastructure

ir_interpreter.cpp has **two debug traces** built in:

```cpp
// Function call trace
lean_trace(*g_interpreter_call, ...)
// e.g.: "call sum [5]" → "ret 15"

// Step trace
lean_trace(*g_interpreter_step, ...)
// e.g.: "  vdecl x_2 : uint32 := lit 0"
//        "  case x_3 : uint8"
//        "    | 0 => ..."
```

Enable with: `set_option trace.interpreter true`

**This is text output, not structured data.** Trace verification requires conversion to a structured format.

### 3.4 Tail-Call Optimization

Important optimization in `eval_body()`'s `vdecl` handling:

```cpp
// For VDecl with FAp where callee is the same function and result is directly returned:
// → Don't create a new frame; overwrite arguments and goto loop start
if (is_self_tail_call(fn, e, b)) {
    // Copy args to parameter slots
    // goto body start
}
```

`sum(UInt32)` does **NOT get this tail-call optimization** (there's `&&&` after `n + sum(n-1)`). However, this affects trace generation for future tail-recursive functions.

---

## 4. Design Choices for Trace Verification

### 4.1 Option A: Lean 4 Fork + ir_interpreter.cpp Modification

**Overview:** Add hooks to `eval_body()` and `eval_expr()` in `ir_interpreter.cpp` to output each step as a structured trace.

**Modifications:**

1. **Add trace buffer:**
```cpp
struct trace_entry {
    enum kind { VDECL, CASE, JMP, RET, CALL, ... };
    kind m_kind;
    var_id m_var;           // target variable
    value m_value;          // result value
    name m_fn;              // called function (for CALL)
    uint8 m_branch_tag;    // branch target (for CASE)
};
std::vector<trace_entry> m_trace;
```

2. **Insert into each eval_body() branch:**
```cpp
case FnBody::VDecl:
    eval_expr(e);
    var(x) = result;
    m_trace.push_back({VDECL, x, result, ...});  // ← added
    body = b;
    break;
```

3. **Output format:** Binary (bincode/protobuf) to stdout or file.

**Pros:**
- Perfect fidelity with Lean's actual evaluation semantics
- Accurately captures results of native function calls (`lean_uint32_add`, etc.)
- Can extend the existing debug trace infrastructure

**Cons:**
- Requires maintaining a Lean 4 fork (version tracking)
- C++ modification and build required (Lean's build is heavy)
- Build system integration for the forked Lean

**Effort:** Large (Lean build environment + C++ modification + build system integration)

### 4.2 Option B: Lean Metaprogram IR Export + Rust IR Interpreter

**Overview:** Write a Lean metaprogram to serialize IR declarations to JSON/bincode. Implement an IR interpreter in Rust to generate traces.

**Step 1: IR Export (Lean side)**

```lean
-- Lean metaprogram to export IR
import Lean.Compiler.IR.Basic

def exportIR (env : Environment) (name : Name) : IO String := do
  match Lean.IR.findEnvDecl env name with
  | some decl => pure (toJson decl)  -- convert IR declaration to JSON
  | none => throw "declaration not found"
```

Lean's `Environment` allows IR declaration access via `findEnvDecl` (EmitC.lean uses this pattern).

**Step 2: Rust IR Interpreter**

```rust
// Rust interpreter that evaluates IR and generates traces
struct IrInterpreter {
    decls: HashMap<FunId, Decl>,
    var_stack: Vec<Value>,
    jp_stack: Vec<FnBodyRef>,
    call_stack: Vec<Frame>,
    trace: Vec<TraceEntry>,
}

impl IrInterpreter {
    fn eval_body(&mut self, body: &FnBody) -> Value { ... }
    fn eval_expr(&mut self, expr: &Expr) -> Value { ... }
    fn call(&mut self, fn_id: &FunId, args: &[Arg]) -> Value { ... }
}
```

**IR subset required for sum(UInt32):**

| Required | Not needed (UInt32 only) |
|----------|------------------------|
| `vdecl`, `case`, `ret` | `ctor`, `reset`, `reuse` |
| `fap` (full application) | `pap`, `ap` (closures) |
| `lit` | `proj`, `uproj`, `sproj` |
| | `box`, `unbox` |
| | `inc`, `dec`, `del` (RC) |
| | `set`, `uset`, `sset` |
| | `jdecl`, `jmp` (join points) |

→ **For UInt32 only, ~30% of the IR is sufficient.**

**Pros:**
- No Lean fork required
- Trace generator and verifier share the same Rust types
- Can incrementally expand IR subset coverage
- Simple build (`cargo build` only)

**Cons:**
- Must accurately reimplement Lean's IR evaluation semantics in Rust
- IR export via Lean metaprogram is unverified (API stability)
- Primitive function semantics (`lean_uint32_add`, etc.) must be manually defined

**Effort:** Medium (IR export + Rust interpreter implementation)

### 4.3 Option C: Generated C Code Instrumentation

**Overview:** Insert trace output into the C code generated by Lean's EmitC.

```c
// EmitC-generated C code (current)
LEAN_EXPORT uint32_t l_sum(uint32_t x_1) {
    uint32_t x_2 = 0;
    uint8_t x_3 = lean_uint32_dec_eq(x_1, x_2);
    if (x_3 == 0) { ... }
}

// After instrumentation
LEAN_EXPORT uint32_t l_sum(uint32_t x_1) {
    TRACE_CALL("l_sum", x_1);
    uint32_t x_2 = 0;
    TRACE_VDECL("x_2", x_2);
    uint8_t x_3 = lean_uint32_dec_eq(x_1, x_2);
    TRACE_VDECL("x_3", x_3);
    if (x_3 == 0) {
        TRACE_CASE(0);
        ...
    }
}
```

**Pros:**
- No Lean modification required
- Achievable via C code post-processing (sed/awk or custom tool)
- Fast execution on host (x86 native)

**Cons:**
- Requires C code parser (depends on EmitC output format)
- IR-level information partially lost
- Fragile (breaks on EmitC output format changes)

**Effort:** Small to medium (C post-processing tool + trace macros)

### 4.4 Comparison Table

| | A: Lean Fork | B: IR Export + Rust | C: C Instrumentation |
|---|---|---|---|
| **Fidelity** | Complete | High (manual reimpl) | Medium (C level) |
| **Lean dependency** | Fork required | Metaprogram only | Depends on EmitC output |
| **Build complexity** | High | Low | Medium |
| **Maintenance cost** | High (version tracking) | Medium | Low to medium |
| **Extensibility** | Highest | High | Low |
| **Initial effort** | Large | Medium | Small |
| **sum(UInt32) feasibility** | Certain | Certain | Certain |
| **ETH2 STF extensibility** | Easy | Requires additional work | Difficult |

---

## 5. Trace Format Design

### 5.1 IR-Level Trace Entries

```rust
enum TraceEntry {
    /// Variable declaration: x := eval(expr) → value
    VDecl {
        var_id: u32,
        ir_type: IrType,
        value: Value,
    },
    /// Function call start
    CallEnter {
        fn_id: FunId,
        args: Vec<Value>,
    },
    /// Function call end
    CallReturn {
        fn_id: FunId,
        result: Value,
    },
    /// Case branch: variable tag value and selected branch
    CaseBranch {
        var_id: u32,
        tag: u32,
    },
    /// Join point jump
    Jump {
        jp_id: u32,
        args: Vec<Value>,
    },
    /// Function return value
    Return {
        value: Value,
    },
    /// Reference count operation (boxed types only)
    RefCount {
        var_id: u32,
        delta: i32,  // +n (inc) or -n (dec)
    },
}

enum Value {
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Bool(bool),
    Obj(ObjSnapshot),  // boxed object snapshot
}
```

### 5.2 Execution Trace Example: sum(5)

```
CallEnter { fn: "l_sum", args: [UInt32(5)] }
  VDecl { var: x_2, type: uint32, value: UInt32(0) }
  VDecl { var: x_3, type: uint8,  value: UInt8(0) }     -- 5 == 0 → false
  CaseBranch { var: x_3, tag: 0 }                        -- false branch
  VDecl { var: x_4, type: uint32, value: UInt32(1) }
  VDecl { var: x_5, type: uint32, value: UInt32(4) }     -- 5 - 1
  CallEnter { fn: "l_sum", args: [UInt32(4)] }
    VDecl { var: x_2, type: uint32, value: UInt32(0) }
    VDecl { var: x_3, type: uint8,  value: UInt8(0) }   -- 4 == 0 → false
    CaseBranch { var: x_3, tag: 0 }
    ... (recursion)
    CallEnter { fn: "l_sum", args: [UInt32(0)] }
      VDecl { var: x_2, type: uint32, value: UInt32(0) }
      VDecl { var: x_3, type: uint8,  value: UInt8(1) } -- 0 == 0 → true
      CaseBranch { var: x_3, tag: 1 }                    -- true branch
      VDecl { var: x_10, type: uint32, value: UInt32(0) }
      Return { value: UInt32(0) }
    CallReturn { fn: "l_sum", result: UInt32(0) }
    ...
  CallReturn { fn: "l_sum", result: UInt32(10) }
  VDecl { var: x_7, type: uint32, value: UInt32(15) }   -- 5 + 10
  VDecl { var: x_8, type: uint32, value: UInt32(65535) }
  VDecl { var: x_9, type: uint32, value: UInt32(15) }   -- 15 & 0xFFFF
  Return { value: UInt32(15) }
CallReturn { fn: "l_sum", result: UInt32(15) }
```

### 5.3 Trace Size Estimates

For `sum(N)`:
- Per call: ~10 entries × ~20 bytes/entry = ~200 bytes
- N recursive calls: ~200N bytes
- `sum(100)`: ~20 KB
- `sum(10000)`: ~2 MB

**Well within acceptable zkVM input size.**

---

## 6. zkVM Guest Verifier Design

### 6.1 What the Verifier Must Check

Input: `(IR declarations, initial args, trace, expected final value)`

For each trace entry:

| Entry Type | Verification |
|-----------|-------------|
| `VDecl(x, ty, val)` | Verify corresponding IR `vdecl` expression. For `fap`, check args are correct and primitive result matches. For `lit`, check value matches. |
| `CallEnter(fn, args)` | Verify callee and arguments match the trace values for the corresponding `fap` |
| `CallReturn(fn, result)` | Verify result matches the last `Return` value of the function body |
| `CaseBranch(var, tag)` | Verify `var`'s value tag matches `tag`. Verify correct branch is selected |
| `Return(val)` | Verify current function body's `ret` variable value matches |
| `Jump(jp, args)` | Verify join point arguments are correct |

### 6.2 Primitive Function Verification

UInt32 primitives are hardcoded in the verifier:

```rust
fn verify_primitive(fn_name: &str, args: &[Value]) -> Value {
    match fn_name {
        "lean_uint32_add"     => UInt32(args[0].as_u32().wrapping_add(args[1].as_u32())),
        "lean_uint32_sub"     => UInt32(args[0].as_u32().wrapping_sub(args[1].as_u32())),
        "lean_uint32_land"    => UInt32(args[0].as_u32() & args[1].as_u32()),
        "lean_uint32_dec_eq"  => UInt8(if args[0].as_u32() == args[1].as_u32() { 1 } else { 0 }),
        _ => panic!("unknown primitive"),
    }
}
```

### 6.3 Estimated Verifier Cycle Count

- Per trace entry verification: ~10-50 cycles (pattern match + arithmetic verification)
- `sum(100)`: ~1000 entries × ~30 cycles = ~30,000 cycles
- Current Lean guest `sum(100)`: 4,946 cycles (UInt32) / 4,124,252 cycles (Nat+Init)

**For UInt32, cycle count is comparable to the current approach. For Nat/Init, dramatic reduction.**
**ELF size would shrink from 1.5MB → ~50KB.**

---

## 7. Open Questions

### 7.1 IR Export Feasibility

- Lean's `findEnvDecl` API provides access to IR declarations, but it's unclear if this is a stable public API
- `EmitC.lean` uses this pattern, so it's at least internally stable
- JSON serialization requires manual implementation (`ToJson` instances may not exist for IR types)

### 7.2 Primitive Function Coverage

- `sum(UInt32)` needs only 4 primitives
- ETH2 STF requires Array, ByteArray, String, Nat, etc. primitives
- Each primitive's exact semantics must be reimplemented in Rust

### 7.3 Boxed Object Tracing

- UInt32 is an unboxed value, so value snapshots are straightforward
- Array, Structure, etc. are boxed objects accessed by reference
- Open question: how much object content to include in traces (all fields vs hash)

### 7.4 Correctness Guarantees

- Can the verifier correctly determine "valid IR execution"? = requires formal IR semantics
- Separate layer from Lean kernel type checking (IR is type-erased)
- Verifier bugs = false proofs possible → verifier correctness is critical

### 7.5 IR Interpreter vs Native Execution Differences

`ir_interpreter.cpp` executes some functions as native code (`interpreter.prefer_native` option). Using native code during trace generation makes those steps invisible.

→ **Trace generation must use pure interpreter mode.**

---

## 8. Proposed Next Steps

### Short-term (PoC)
1. **Option B (Rust IR interpreter)** for sum(UInt32) PoC
2. Hand-code IR declarations in Rust (defer JSON export)
3. Validate the trace generation → zkVM verification pipeline

### Medium-term (IR Export Automation)
4. Lean metaprogram to export IR as JSON
5. Rust side reads JSON and feeds it to the IR interpreter
6. Expand IR subset (join points, boxed types, RC)

### Long-term (Production)
7. **Migrate to Option A (Lean fork)** for higher-fidelity traces
8. Support complex computations like ETH2 STF
9. Formal verification of the verifier (prove verifier correctness in Lean?)
