#![no_main]
risc0_zkvm::guest::entry!(main);

use risc0_zkvm::guest::env;

mod types;
mod transition;

fn main() {
    let input: Vec<u8> = env::read();
    let result = match types::BeaconState::deserialize(&input) {
        Some((pre_state, offset)) => {
            match types::SignedBeaconBlock::deserialize(&input[offset..]) {
                Some((signed_block, _)) => {
                    match transition::state_transition(pre_state, &signed_block) {
                        Ok(post_state) => post_state.serialize(),
                        Err(_) => vec![0xFD], // STF error
                    }
                }
                None => vec![0xFE], // Block decode error
            }
        }
        None => vec![0xFF], // State decode error
    };
    env::commit_slice(&result);
}
