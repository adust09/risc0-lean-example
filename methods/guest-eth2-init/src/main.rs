#![no_main]
risc0_zkvm::guest::entry!(main);

use risc0_zkvm::guest::env;

extern "C" {
    fn lean_eth2_init_entry(
        input: *const u8,
        input_len: usize,
        output: *mut *mut u8,
        output_len: *mut usize,
    );
}

fn main() {
    let input: Vec<u8> = env::read();
    let mut output_ptr: *mut u8 = std::ptr::null_mut();
    let mut output_len: usize = 0;

    unsafe {
        lean_eth2_init_entry(
            input.as_ptr(),
            input.len(),
            &mut output_ptr,
            &mut output_len,
        );
    }

    let result = unsafe { std::slice::from_raw_parts(output_ptr, output_len) };
    env::commit_slice(result);
}
