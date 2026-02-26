#![no_main]
risc0_zkvm::guest::entry!(main);

use risc0_zkvm::guest::env;

// Exact same algorithm as guest/Guest/Basic.lean:
// partial def sum (n : Nat) : Nat :=
//   if n == 0 then 0 else (n + sum (n - 1)) &&& 0xFFFF
fn sum(n: u32) -> u32 {
    if n == 0 {
        0
    } else {
        (n + sum(n - 1)) & 0xFFFF
    }
}

fn main() {
    let input: u32 = env::read();
    let result = sum(input);
    env::commit(&result);
}
