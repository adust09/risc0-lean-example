/// Ethereum Consensus Layer state transition — Pure Rust.
/// Mirrors the Lean Eth2.Transition module.

use crate::types::*;

pub fn state_transition(
    state: BeaconState,
    signed_block: &SignedBeaconBlock,
) -> Result<BeaconState, &'static str> {
    let block = &signed_block.message;
    let state = process_slots(state, block.slot)?;
    let state = process_block(state, block)?;
    Ok(state)
}

fn process_slots(mut state: BeaconState, target_slot: Slot) -> Result<BeaconState, &'static str> {
    if target_slot <= state.slot {
        return Err("target_slot <= state.slot");
    }
    while state.slot < target_slot {
        state = process_slot(state);
        if (state.slot + 1) % SLOTS_PER_EPOCH == 0 {
            state = process_epoch(state);
        }
        state.slot += 1;
    }
    Ok(state)
}

fn process_slot(mut state: BeaconState) -> BeaconState {
    let stub_root = vec![0u8; 32];
    let idx = (state.slot % SLOTS_PER_HISTORICAL_ROOT) as usize;
    if idx < state.state_roots.len() {
        state.state_roots[idx] = stub_root.clone();
    }
    if state.latest_block_header.state_root == vec![0u8; 32] {
        state.latest_block_header.state_root = stub_root.clone();
    }
    if idx < state.block_roots.len() {
        state.block_roots[idx] = stub_root;
    }
    state
}

// ── Epoch processing ─────────────────────────────

fn compute_epoch_at_slot(slot: Slot) -> Epoch { slot / SLOTS_PER_EPOCH }
fn get_current_epoch(state: &BeaconState) -> Epoch { compute_epoch_at_slot(state.slot) }
fn get_previous_epoch(state: &BeaconState) -> Epoch {
    let ce = get_current_epoch(state);
    if ce > 0 { ce - 1 } else { ce }
}

fn is_active_validator(v: &Validator, epoch: Epoch) -> bool {
    v.activation_epoch <= epoch && epoch < v.exit_epoch
}

fn get_active_validator_indices(state: &BeaconState, epoch: Epoch) -> Vec<usize> {
    (0..state.validators.len())
        .filter(|&i| is_active_validator(&state.validators[i], epoch))
        .collect()
}

fn get_total_active_balance(state: &BeaconState) -> Gwei {
    let epoch = get_current_epoch(state);
    let total: Gwei = get_active_validator_indices(state, epoch)
        .iter()
        .map(|&i| state.validators[i].effective_balance)
        .sum();
    total.max(EFFECTIVE_BALANCE_INCREMENT)
}

fn integer_squareroot(n: u64) -> u64 {
    if n == 0 { return 0; }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x { x = y; y = (x + n / x) / 2; }
    x
}

fn get_base_reward_per_increment(state: &BeaconState) -> Gwei {
    let total = get_total_active_balance(state);
    let sqrt = integer_squareroot(total);
    if sqrt == 0 { 0 } else { EFFECTIVE_BALANCE_INCREMENT * BASE_REWARD_FACTOR / sqrt }
}

fn get_base_reward(state: &BeaconState, index: usize) -> Gwei {
    if index >= state.validators.len() { return 0; }
    let increments = state.validators[index].effective_balance / EFFECTIVE_BALANCE_INCREMENT;
    increments * get_base_reward_per_increment(state)
}

fn has_flag(flags: u8, index: usize) -> bool { (flags >> index) & 1 == 1 }

fn is_in_inactivity_leak(state: &BeaconState) -> bool {
    get_current_epoch(state) > state.finalized_checkpoint.epoch + MIN_EPOCHS_TO_INACTIVITY_PENALTY
}

fn process_epoch(mut state: BeaconState) -> BeaconState {
    let current_epoch = get_current_epoch(&state);
    if current_epoch <= 1 { /* skip justification */ }

    // Inactivity updates
    if current_epoch > 0 {
        let active = get_active_validator_indices(&state, current_epoch);
        let in_leak = is_in_inactivity_leak(&state);
        for &i in &active {
            if i < state.inactivity_scores.len() {
                let participated = i < state.previous_epoch_participation.len()
                    && has_flag(state.previous_epoch_participation[i], TIMELY_TARGET_FLAG_INDEX);
                if participated {
                    state.inactivity_scores[i] = state.inactivity_scores[i].saturating_sub(INACTIVITY_SCORE_RECOVERY_RATE);
                } else if in_leak {
                    state.inactivity_scores[i] += INACTIVITY_SCORE_BIAS;
                }
            }
        }
    }

    // Rewards and penalties (simplified)
    if current_epoch > 0 {
        let prev_epoch = get_previous_epoch(&state);
        let active = get_active_validator_indices(&state, prev_epoch);
        let total_active = get_total_active_balance(&state);
        let in_leak = is_in_inactivity_leak(&state);

        for &i in &active {
            let base_reward = get_base_reward(&state, i);
            let flags = if i < state.previous_epoch_participation.len() {
                state.previous_epoch_participation[i]
            } else { 0 };

            for &(flag_idx, weight) in &[
                (TIMELY_SOURCE_FLAG_INDEX, TIMELY_SOURCE_WEIGHT),
                (TIMELY_TARGET_FLAG_INDEX, TIMELY_TARGET_WEIGHT),
                (TIMELY_HEAD_FLAG_INDEX, TIMELY_HEAD_WEIGHT),
            ] {
                if has_flag(flags, flag_idx) {
                    if !in_leak && i < state.balances.len() {
                        let reward = base_reward * weight / WEIGHT_DENOMINATOR;
                        state.balances[i] = state.balances[i].saturating_add(reward);
                    }
                } else if i < state.balances.len() {
                    let penalty = base_reward * weight / WEIGHT_DENOMINATOR;
                    state.balances[i] = state.balances[i].saturating_sub(penalty);
                }
            }

            // Inactivity penalty
            if !has_flag(flags, TIMELY_TARGET_FLAG_INDEX) {
                if i < state.inactivity_scores.len() && i < state.balances.len() {
                    let penalty = state.validators[i].effective_balance
                        * state.inactivity_scores[i] / INACTIVITY_PENALTY_QUOTIENT_BELLATRIX;
                    state.balances[i] = state.balances[i].saturating_sub(penalty);
                }
            }
        }
    }

    // Effective balance updates
    for i in 0..state.validators.len() {
        if i < state.balances.len() {
            let balance = state.balances[i];
            let eff = state.validators[i].effective_balance;
            let down = EFFECTIVE_BALANCE_INCREMENT / 4;
            let up = EFFECTIVE_BALANCE_INCREMENT * 5 / 4;
            if balance + down < eff || eff + up < balance {
                state.validators[i].effective_balance =
                    (balance - balance % EFFECTIVE_BALANCE_INCREMENT).min(MAX_EFFECTIVE_BALANCE);
            }
        }
    }

    // Participation flag rotation
    state.previous_epoch_participation = state.current_epoch_participation.clone();
    state.current_epoch_participation = vec![0u8; state.validators.len()];

    // Resets
    let next_epoch = current_epoch + 1;
    if next_epoch % EPOCHS_PER_ETH1_VOTING_PERIOD == 0 {
        state.eth1_data_votes.clear();
    }
    let slashings_idx = (next_epoch % EPOCHS_PER_SLASHINGS_VECTOR) as usize;
    if slashings_idx < state.slashings.len() {
        state.slashings[slashings_idx] = 0;
    }
    let mix_idx = (next_epoch % EPOCHS_PER_HISTORICAL_VECTOR) as usize;
    let current_mix_idx = (current_epoch % EPOCHS_PER_HISTORICAL_VECTOR) as usize;
    if mix_idx < state.randao_mixes.len() && current_mix_idx < state.randao_mixes.len() {
        let mix = state.randao_mixes[current_mix_idx].clone();
        state.randao_mixes[mix_idx] = mix;
    }

    state
}

// ── Block processing ─────────────────────────────

fn process_block(mut state: BeaconState, block: &BeaconBlock) -> Result<BeaconState, &'static str> {
    // Block header
    if block.slot != state.slot { return Err("block.slot != state.slot"); }
    let header = BeaconBlockHeader {
        slot: block.slot,
        proposer_index: block.proposer_index,
        parent_root: block.parent_root.clone(),
        state_root: vec![0u8; 32],
        body_root: vec![0u8; 32],
    };
    state.latest_block_header = header;

    // RANDAO
    let current_epoch = get_current_epoch(&state);
    let idx = (current_epoch % EPOCHS_PER_HISTORICAL_VECTOR) as usize;
    if idx < state.randao_mixes.len() {
        // Stub: just use hash of reveal
        state.randao_mixes[idx] = vec![0u8; 32]; // stub
    }

    // Eth1 data vote
    state.eth1_data_votes.push(block.body.eth1_data.clone());

    // Execution payload header (stub)
    state.latest_execution_payload_header = ExecutionPayloadHeader {
        parent_hash: block.body.execution_payload.parent_hash.clone(),
        fee_recipient: block.body.execution_payload.fee_recipient.clone(),
        state_root: block.body.execution_payload.state_root.clone(),
        receipts_root: block.body.execution_payload.receipts_root.clone(),
        logs_bloom: block.body.execution_payload.logs_bloom.clone(),
        prev_randao: block.body.execution_payload.prev_randao.clone(),
        block_number: block.body.execution_payload.block_number,
        gas_limit: block.body.execution_payload.gas_limit,
        gas_used: block.body.execution_payload.gas_used,
        timestamp: block.body.execution_payload.timestamp,
        extra_data: block.body.execution_payload.extra_data.clone(),
        base_fee_per_gas: block.body.execution_payload.base_fee_per_gas,
        block_hash: block.body.execution_payload.block_hash.clone(),
        transactions_root: vec![0u8; 32],
        withdrawals_root: vec![0u8; 32],
    };

    Ok(state)
}
