/**
 * ETH2 STF guest wrapper — NO Init initialization.
 *
 * Calls risc0_main_eth2() directly without initialize_Guest().
 * This tests whether Lean compiled code works without runtime initialization.
 */
#include <lean/lean.h>
#include <stdlib.h>
#include <string.h>

/* Lean-exported entry point: ByteArray → ByteArray */
extern lean_object* risc0_main_eth2(lean_object*);

/**
 * Convert a C byte buffer to a Lean ByteArray object.
 */
static lean_object* bytes_to_lean(const uint8_t* data, size_t len) {
    lean_object* arr = lean_alloc_sarray(1, len, len);
    memcpy(lean_sarray_cptr(arr), data, len);
    return arr;
}

/**
 * Entry point called from Rust guest via FFI.
 * Takes raw bytes, calls Lean STF, returns raw bytes.
 */
void lean_eth2_noinit_entry(const uint8_t* input, size_t input_len,
                             uint8_t** output, size_t* output_len) {
    /* Do NOT call initialize_Guest() — that's the experiment */
    lean_object* lean_input = bytes_to_lean(input, input_len);
    lean_object* lean_result = risc0_main_eth2(lean_input);

    /* Extract bytes from result ByteArray */
    *output_len = lean_sarray_size(lean_result);
    *output = (uint8_t*)lean_sarray_cptr(lean_result);
}
