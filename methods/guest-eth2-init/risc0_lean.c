/**
 * ETH2 STF guest wrapper — WITH Init initialization.
 *
 * Workaround: initialize_Init_Data() fails in zkVM (strerror(0) returns
 * "success" but the init function treats errno=0 as error).  Pre-calling
 * it sets _G_initialized=true so that initialize_Init() skips it and succeeds.
 */
#include <lean/lean.h>
#include <stdlib.h>
#include <string.h>

extern lean_object* lean_initialize_runtime_module(lean_object*);
extern lean_object* initialize_Init_Data(uint8_t, lean_object*);
extern lean_object* initialize_Init(uint8_t, lean_object*);
extern lean_object* initialize_Guest(uint8_t, lean_object*);
extern lean_object* risc0_main_eth2(lean_object*);

/**
 * Entry point called from Rust guest via FFI.
 */
void lean_eth2_init_entry(const uint8_t* input, size_t input_len,
                           uint8_t** output, size_t* output_len) {
    /* Step 1: Initialize runtime */
    lean_object* res = lean_initialize_runtime_module(lean_io_mk_world());
    if (lean_io_result_is_ok(res)) lean_dec_ref(res);

    /* Step 2: Workaround — pre-call Init_Data (fails but sets _G_initialized) */
    res = initialize_Init_Data(1, lean_io_mk_world());
    /* Ignore result — failure is expected */

    /* Step 3: Full Init (succeeds because Data flag is already set) */
    res = initialize_Init(1, lean_io_mk_world());
    if (lean_io_result_is_ok(res)) lean_dec_ref(res);

    /* Step 4: Initialize Guest */
    res = initialize_Guest(1, lean_io_mk_world());
    if (lean_io_result_is_ok(res)) lean_dec_ref(res);

    /* Step 5: Create ByteArray from real input and call risc0_main_eth2 */
    lean_object* lean_input = lean_alloc_sarray(1, input_len, input_len);
    memcpy(lean_sarray_cptr(lean_input), input, input_len);

    lean_object* lean_result = risc0_main_eth2(lean_input);

    /* Step 6: Return result */
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
