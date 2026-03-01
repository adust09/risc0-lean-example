/**
 * ETH2 STF guest wrapper — WITH selective Init initialization.
 *
 * Overrides the monolithic initialize_Init() (392 modules, ~15M cycles)
 * with a selective version that only initializes modules whose BSS symbols
 * are actually referenced by Guest code:
 *   - l_ByteArray_empty        (Init.Data.ByteArray.Basic)
 *   - l_Int_instInhabited       (Init.Data.Int.Basic)
 *   - l_instInhabitedUInt64     (Init.Prelude)
 *   - l_instInhabitedUInt8      (Init.Prelude)
 *
 * Strategy: call initialize_Init_Prelude (covers instInhabitedUInt64/UInt8
 * plus essential runtime globals), then manually construct the remaining two
 * symbols to avoid the deep transitive dependency chains of
 * Init.Data.ByteArray.Basic and Init.Data.Int.Basic.
 *
 * The --allow-multiple-definition linker flag ensures this version
 * overrides the one from libInit.a.
 */
#include <lean/lean.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

extern lean_object* lean_initialize_runtime_module(lean_object*);
extern lean_object* initialize_Guest(uint8_t, lean_object*);
extern lean_object* risc0_main_eth2(lean_object*);

/* Sub-module init function from libInit.a */
extern lean_object* initialize_Init_Prelude(uint8_t, lean_object*);

/* BSS symbols that need initialization but whose full init chains are too heavy */
extern lean_object* l_ByteArray_empty;
extern lean_object* l_Int_instInhabited;

/**
 * Selective override of initialize_Init().
 *
 * Calls only Init_Prelude (provides l_instInhabitedUInt64, l_instInhabitedUInt8,
 * and other essential runtime globals), then manually constructs the remaining
 * two BSS symbols to avoid pulling in hundreds of transitive Init modules.
 */
static bool _selective_init_done = false;
LEAN_EXPORT lean_object* initialize_Init(uint8_t builtin, lean_object* w) {
    if (_selective_init_done) return lean_io_result_mk_ok(lean_box(0));
    _selective_init_done = true;

    lean_object* res;

    /* Init.Prelude — provides l_instInhabitedUInt64, l_instInhabitedUInt8,
       and essential runtime globals (Option.none, Bool ctors, etc.) */
    res = initialize_Init_Prelude(builtin, lean_io_mk_world());
    if (lean_io_result_is_error(res)) return res;
    lean_dec_ref(res);

    /* Manually construct l_ByteArray_empty (empty scalar array, elem size 1) */
    l_ByteArray_empty = lean_alloc_sarray(1, 0, 0);
    lean_mark_persistent(l_ByteArray_empty);

    /* Manually construct l_Int_instInhabited (Inhabited Int = ⟨0⟩, Int.ofNat 0 = lean_box(0)) */
    l_Int_instInhabited = lean_box(0);

    return lean_io_result_mk_ok(lean_box(0));
}

/**
 * Entry point called from Rust guest via FFI.
 */
void lean_eth2_init_entry(const uint8_t* input, size_t input_len,
                           uint8_t** output, size_t* output_len) {
    /* Step 1: Initialize runtime */
    lean_object* res = lean_initialize_runtime_module(lean_io_mk_world());
    if (lean_io_result_is_ok(res)) lean_dec_ref(res);

    /* Step 2: Initialize Guest (calls our selective initialize_Init internally) */
    res = initialize_Guest(1, lean_io_mk_world());
    if (lean_io_result_is_ok(res)) lean_dec_ref(res);

    /* Step 3: Create ByteArray from real input and call risc0_main_eth2 */
    lean_object* lean_input = lean_alloc_sarray(1, input_len, input_len);
    memcpy(lean_sarray_cptr(lean_input), input, input_len);

    lean_object* lean_result = risc0_main_eth2(lean_input);

    /* Step 4: Return result */
    if (lean_result == NULL || lean_is_scalar(lean_result)) {
        /* Error fallback */
        static uint8_t err_buf[2] = {0xDE, 0xAD};
        *output = err_buf;
        *output_len = 2;
    } else {
        *output_len = lean_sarray_size(lean_result);
        *output = (uint8_t*)lean_sarray_cptr(lean_result);
    }
}
