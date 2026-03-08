//! Generate ETH2 test input (BeaconState + SignedBeaconBlock) in the same binary format
//! used by the Lean/Rust guest serializers.

use std::fs;

fn main() {
    let num_validators: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let output_path = std::env::args().nth(2).unwrap_or_else(|| "eth2_input.bin".to_string());

    let data = build_test_input(num_validators);
    fs::write(&output_path, &data).expect("Failed to write output");
    eprintln!(
        "Generated ETH2 test input: {} bytes, {} validators -> {}",
        data.len(),
        num_validators,
        output_path
    );
}

const FAR_FUTURE_EPOCH: u64 = u64::MAX;

fn write_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn write_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn write_u8(buf: &mut Vec<u8>, v: u8) {
    buf.push(v);
}
fn write_bool(buf: &mut Vec<u8>, v: bool) {
    buf.push(if v { 1 } else { 0 });
}
fn write_bytes(buf: &mut Vec<u8>, v: &[u8]) {
    write_u32(buf, v.len() as u32);
    buf.extend_from_slice(v);
}
fn zero_bytes(n: usize) -> Vec<u8> {
    vec![0u8; n]
}

fn build_test_input(num_validators: usize) -> Vec<u8> {
    let mut buf = Vec::new();

    // BeaconState
    write_u64(&mut buf, 1_000_000); // genesis_time
    write_bytes(&mut buf, &zero_bytes(32)); // genesis_validators_root
    write_u64(&mut buf, 100); // slot

    // fork
    write_bytes(&mut buf, &[0, 0, 0, 0]);
    write_bytes(&mut buf, &[1, 0, 0, 0]);
    write_u64(&mut buf, 0);

    // latest_block_header
    write_u64(&mut buf, 100);
    write_u64(&mut buf, 0);
    write_bytes(&mut buf, &zero_bytes(32));
    write_bytes(&mut buf, &zero_bytes(32));
    write_bytes(&mut buf, &zero_bytes(32));

    // block_roots
    let roots_len = 200u32;
    write_u32(&mut buf, roots_len);
    for _ in 0..roots_len {
        write_bytes(&mut buf, &zero_bytes(32));
    }

    // state_roots
    write_u32(&mut buf, roots_len);
    for _ in 0..roots_len {
        write_bytes(&mut buf, &zero_bytes(32));
    }

    // historical_roots
    write_u32(&mut buf, 0);

    // eth1_data
    write_bytes(&mut buf, &zero_bytes(32));
    write_u64(&mut buf, 0);
    write_bytes(&mut buf, &zero_bytes(32));

    // eth1_data_votes
    write_u32(&mut buf, 0);

    // eth1_deposit_index
    write_u64(&mut buf, 0);

    // validators
    let n = num_validators as u32;
    write_u32(&mut buf, n);
    for i in 0..num_validators {
        let mut pk = vec![0u8; 48];
        pk[0] = i as u8;
        pk[1] = (i >> 8) as u8;
        write_bytes(&mut buf, &pk);
        write_bytes(&mut buf, &zero_bytes(32));
        write_u64(&mut buf, 32_000_000_000);
        write_bool(&mut buf, false);
        write_u64(&mut buf, 0);
        write_u64(&mut buf, 0);
        write_u64(&mut buf, FAR_FUTURE_EPOCH);
        write_u64(&mut buf, FAR_FUTURE_EPOCH);
    }

    // balances
    write_u32(&mut buf, n);
    for _ in 0..num_validators {
        write_u64(&mut buf, 32_000_000_000);
    }

    // randao_mixes
    let mixes_len = 200u32;
    write_u32(&mut buf, mixes_len);
    for _ in 0..mixes_len {
        write_bytes(&mut buf, &zero_bytes(32));
    }

    // slashings
    let slashings_len = 200u32;
    write_u32(&mut buf, slashings_len);
    for _ in 0..slashings_len {
        write_u64(&mut buf, 0);
    }

    // previous_epoch_participation
    write_u32(&mut buf, n);
    for _ in 0..num_validators {
        write_u8(&mut buf, 0x07);
    }

    // current_epoch_participation
    write_u32(&mut buf, n);
    for _ in 0..num_validators {
        write_u8(&mut buf, 0x07);
    }

    // justification_bits
    write_bytes(&mut buf, &[0, 0, 0, 0]);

    // checkpoints
    write_u64(&mut buf, 0);
    write_bytes(&mut buf, &zero_bytes(32));
    write_u64(&mut buf, 0);
    write_bytes(&mut buf, &zero_bytes(32));
    write_u64(&mut buf, 0);
    write_bytes(&mut buf, &zero_bytes(32));

    // inactivity_scores
    write_u32(&mut buf, n);
    for _ in 0..num_validators {
        write_u64(&mut buf, 0);
    }

    // current_sync_committee
    let sync_committee_size = 512u32;
    write_u32(&mut buf, sync_committee_size);
    for _ in 0..sync_committee_size {
        write_bytes(&mut buf, &zero_bytes(48));
    }
    write_bytes(&mut buf, &zero_bytes(48));

    // next_sync_committee
    write_u32(&mut buf, sync_committee_size);
    for _ in 0..sync_committee_size {
        write_bytes(&mut buf, &zero_bytes(48));
    }
    write_bytes(&mut buf, &zero_bytes(48));

    // latest_execution_payload_header
    write_bytes(&mut buf, &zero_bytes(32));
    write_bytes(&mut buf, &zero_bytes(20));
    write_bytes(&mut buf, &zero_bytes(32));
    write_bytes(&mut buf, &zero_bytes(32));
    write_bytes(&mut buf, &zero_bytes(256));
    write_bytes(&mut buf, &zero_bytes(32));
    write_u64(&mut buf, 0);
    write_u64(&mut buf, 0);
    write_u64(&mut buf, 0);
    write_u64(&mut buf, 0);
    write_bytes(&mut buf, &[]);
    write_u64(&mut buf, 0);
    write_bytes(&mut buf, &zero_bytes(32));
    write_bytes(&mut buf, &zero_bytes(32));
    write_bytes(&mut buf, &zero_bytes(32));

    // next_withdrawal_index, next_withdrawal_validator_index
    write_u64(&mut buf, 0);
    write_u64(&mut buf, 0);

    // historical_summaries
    write_u32(&mut buf, 0);

    // SignedBeaconBlock
    write_u64(&mut buf, 101); // slot
    write_u64(&mut buf, 101 % num_validators as u64); // proposer_index
    write_bytes(&mut buf, &zero_bytes(32)); // parent_root
    write_bytes(&mut buf, &zero_bytes(32)); // state_root
    write_bytes(&mut buf, &zero_bytes(96)); // randao_reveal
    write_bytes(&mut buf, &zero_bytes(32)); // eth1_data.deposit_root
    write_u64(&mut buf, 0); // eth1_data.deposit_count
    write_bytes(&mut buf, &zero_bytes(32)); // eth1_data.block_hash
    write_bytes(&mut buf, &zero_bytes(32)); // graffiti
    write_u32(&mut buf, 0); // op_count
    write_bytes(&mut buf, &zero_bytes(96)); // signature

    buf
}
