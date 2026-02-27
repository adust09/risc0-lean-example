/// Ethereum Consensus Layer types — Pure Rust equivalent of the Lean types.
/// Mirrors the Lean Eth2.Types and Eth2.Containers modules.

pub type Slot = u64;
pub type Epoch = u64;
pub type Gwei = u64;
pub type ValidatorIndex = u64;
pub type WithdrawalIndex = u64;
pub type Root = Vec<u8>;
pub type Bytes32 = Vec<u8>;
pub type BLSPubkey = Vec<u8>;
pub type BLSSignature = Vec<u8>;
pub type ParticipationFlags = u8;

pub const FAR_FUTURE_EPOCH: Epoch = u64::MAX;
pub const SLOTS_PER_EPOCH: u64 = 32;
pub const SLOTS_PER_HISTORICAL_ROOT: u64 = 8192;
pub const EPOCHS_PER_HISTORICAL_VECTOR: u64 = 65536;
pub const EPOCHS_PER_SLASHINGS_VECTOR: u64 = 8192;
pub const EPOCHS_PER_ETH1_VOTING_PERIOD: u64 = 64;
pub const EPOCHS_PER_SYNC_COMMITTEE_PERIOD: u64 = 256;
pub const MAX_EFFECTIVE_BALANCE: Gwei = 32_000_000_000;
pub const EFFECTIVE_BALANCE_INCREMENT: Gwei = 1_000_000_000;
pub const MIN_EPOCHS_TO_INACTIVITY_PENALTY: u64 = 4;
pub const BASE_REWARD_FACTOR: u64 = 64;
pub const TIMELY_SOURCE_FLAG_INDEX: usize = 0;
pub const TIMELY_TARGET_FLAG_INDEX: usize = 1;
pub const TIMELY_HEAD_FLAG_INDEX: usize = 2;
pub const TIMELY_SOURCE_WEIGHT: u64 = 14;
pub const TIMELY_TARGET_WEIGHT: u64 = 26;
pub const TIMELY_HEAD_WEIGHT: u64 = 14;
pub const WEIGHT_DENOMINATOR: u64 = 64;
pub const PROPOSER_WEIGHT: u64 = 8;
pub const SYNC_REWARD_WEIGHT: u64 = 2;
pub const INACTIVITY_PENALTY_QUOTIENT_BELLATRIX: u64 = 16_777_216;
pub const INACTIVITY_SCORE_BIAS: u64 = 4;
pub const INACTIVITY_SCORE_RECOVERY_RATE: u64 = 16;
pub const MIN_SLASHING_PENALTY_QUOTIENT_BELLATRIX: u64 = 32;
pub const PROPORTIONAL_SLASHING_MULTIPLIER_BELLATRIX: u64 = 3;
pub const WHISTLEBLOWER_REWARD_QUOTIENT: u64 = 512;
pub const MAX_VALIDATORS_PER_WITHDRAWALS_SWEEP: u64 = 16384;
pub const MAX_WITHDRAWALS_PER_PAYLOAD: usize = 16;
pub const MIN_VALIDATOR_WITHDRAWABILITY_DELAY: u64 = 256;
pub const CHURN_LIMIT_QUOTIENT: u64 = 65536;
pub const MIN_PER_EPOCH_CHURN_LIMIT: u64 = 4;
pub const EJECTION_BALANCE: Gwei = 16_000_000_000;
pub const SHARD_COMMITTEE_PERIOD: u64 = 256;
pub const SYNC_COMMITTEE_SIZE: usize = 512;
pub const MIN_ATTESTATION_INCLUSION_DELAY: u64 = 1;

#[derive(Clone, Default)]
pub struct Fork {
    pub previous_version: Vec<u8>,
    pub current_version: Vec<u8>,
    pub epoch: Epoch,
}

#[derive(Clone, Default)]
pub struct Checkpoint {
    pub epoch: Epoch,
    pub root: Root,
}

#[derive(Clone, Default)]
pub struct Validator {
    pub pubkey: BLSPubkey,
    pub withdrawal_credentials: Bytes32,
    pub effective_balance: Gwei,
    pub slashed: bool,
    pub activation_eligibility_epoch: Epoch,
    pub activation_epoch: Epoch,
    pub exit_epoch: Epoch,
    pub withdrawable_epoch: Epoch,
}

#[derive(Clone, Default)]
pub struct Eth1Data {
    pub deposit_root: Root,
    pub deposit_count: u64,
    pub block_hash: Vec<u8>,
}

#[derive(Clone, Default)]
pub struct BeaconBlockHeader {
    pub slot: Slot,
    pub proposer_index: ValidatorIndex,
    pub parent_root: Root,
    pub state_root: Root,
    pub body_root: Root,
}

#[derive(Clone, Default)]
pub struct SyncCommittee {
    pub pubkeys: Vec<BLSPubkey>,
    pub aggregate_pubkey: BLSPubkey,
}

#[derive(Clone, Default)]
pub struct SyncAggregate {
    pub sync_committee_bits: Vec<u8>,
    pub sync_committee_signature: BLSSignature,
}

#[derive(Clone, Default)]
pub struct ExecutionPayloadHeader {
    pub parent_hash: Vec<u8>,
    pub fee_recipient: Vec<u8>,
    pub state_root: Bytes32,
    pub receipts_root: Bytes32,
    pub logs_bloom: Vec<u8>,
    pub prev_randao: Bytes32,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: Vec<u8>,
    pub base_fee_per_gas: u64,
    pub block_hash: Vec<u8>,
    pub transactions_root: Root,
    pub withdrawals_root: Root,
}

#[derive(Clone, Default)]
pub struct ExecutionPayload {
    pub parent_hash: Vec<u8>,
    pub fee_recipient: Vec<u8>,
    pub state_root: Bytes32,
    pub receipts_root: Bytes32,
    pub logs_bloom: Vec<u8>,
    pub prev_randao: Bytes32,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: Vec<u8>,
    pub base_fee_per_gas: u64,
    pub block_hash: Vec<u8>,
    pub transactions: Vec<Vec<u8>>,
    pub withdrawals: Vec<Withdrawal>,
}

#[derive(Clone, Default)]
pub struct Withdrawal {
    pub index: WithdrawalIndex,
    pub validator_index: ValidatorIndex,
    pub address: Vec<u8>,
    pub amount: Gwei,
}

#[derive(Clone, Default)]
pub struct HistoricalSummary {
    pub block_summary_root: Root,
    pub state_summary_root: Root,
}

#[derive(Clone, Default)]
pub struct BeaconBlockBody {
    pub randao_reveal: BLSSignature,
    pub eth1_data: Eth1Data,
    pub graffiti: Bytes32,
    pub execution_payload: ExecutionPayload,
    pub sync_aggregate: SyncAggregate,
}

#[derive(Clone, Default)]
pub struct BeaconBlock {
    pub slot: Slot,
    pub proposer_index: ValidatorIndex,
    pub parent_root: Root,
    pub state_root: Root,
    pub body: BeaconBlockBody,
}

#[derive(Clone, Default)]
pub struct SignedBeaconBlock {
    pub message: BeaconBlock,
    pub signature: BLSSignature,
}

#[derive(Clone, Default)]
pub struct BeaconState {
    pub genesis_time: u64,
    pub genesis_validators_root: Root,
    pub slot: Slot,
    pub fork: Fork,
    pub latest_block_header: BeaconBlockHeader,
    pub block_roots: Vec<Root>,
    pub state_roots: Vec<Root>,
    pub historical_roots: Vec<Root>,
    pub eth1_data: Eth1Data,
    pub eth1_data_votes: Vec<Eth1Data>,
    pub eth1_deposit_index: u64,
    pub validators: Vec<Validator>,
    pub balances: Vec<Gwei>,
    pub randao_mixes: Vec<Bytes32>,
    pub slashings: Vec<Gwei>,
    pub previous_epoch_participation: Vec<ParticipationFlags>,
    pub current_epoch_participation: Vec<ParticipationFlags>,
    pub justification_bits: Vec<u8>,
    pub previous_justified_checkpoint: Checkpoint,
    pub current_justified_checkpoint: Checkpoint,
    pub finalized_checkpoint: Checkpoint,
    pub inactivity_scores: Vec<u64>,
    pub current_sync_committee: SyncCommittee,
    pub next_sync_committee: SyncCommittee,
    pub latest_execution_payload_header: ExecutionPayloadHeader,
    pub next_withdrawal_index: WithdrawalIndex,
    pub next_withdrawal_validator_index: ValidatorIndex,
    pub historical_summaries: Vec<HistoricalSummary>,
}

// ── Serialization helpers ────────────────────────

fn read_u64(data: &[u8], off: usize) -> Option<(u64, usize)> {
    if off + 8 > data.len() { return None; }
    let v = u64::from_le_bytes(data[off..off+8].try_into().ok()?);
    Some((v, off + 8))
}

fn read_u32(data: &[u8], off: usize) -> Option<(u32, usize)> {
    if off + 4 > data.len() { return None; }
    let v = u32::from_le_bytes(data[off..off+4].try_into().ok()?);
    Some((v, off + 4))
}

fn read_u8(data: &[u8], off: usize) -> Option<(u8, usize)> {
    if off >= data.len() { return None; }
    Some((data[off], off + 1))
}

fn read_bool(data: &[u8], off: usize) -> Option<(bool, usize)> {
    let (b, off) = read_u8(data, off)?;
    Some((b != 0, off))
}

fn read_bytes(data: &[u8], off: usize) -> Option<(Vec<u8>, usize)> {
    let (len, off) = read_u32(data, off)?;
    let n = len as usize;
    if off + n > data.len() { return None; }
    Some((data[off..off+n].to_vec(), off + n))
}

fn read_array<T>(data: &[u8], off: usize, reader: fn(&[u8], usize) -> Option<(T, usize)>) -> Option<(Vec<T>, usize)> {
    let (count, mut off) = read_u32(data, off)?;
    let mut arr = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let (item, new_off) = reader(data, off)?;
        arr.push(item);
        off = new_off;
    }
    Some((arr, off))
}

fn write_u64(buf: &mut Vec<u8>, v: u64) { buf.extend_from_slice(&v.to_le_bytes()); }
fn write_u32(buf: &mut Vec<u8>, v: u32) { buf.extend_from_slice(&v.to_le_bytes()); }
fn write_u8(buf: &mut Vec<u8>, v: u8) { buf.push(v); }
fn write_bool(buf: &mut Vec<u8>, v: bool) { buf.push(if v { 1 } else { 0 }); }
fn write_bytes(buf: &mut Vec<u8>, v: &[u8]) { write_u32(buf, v.len() as u32); buf.extend_from_slice(v); }

// ── Deserialize helpers for containers ───────────

fn read_fork(data: &[u8], off: usize) -> Option<(Fork, usize)> {
    let (pv, off) = read_bytes(data, off)?;
    let (cv, off) = read_bytes(data, off)?;
    let (e, off) = read_u64(data, off)?;
    Some((Fork { previous_version: pv, current_version: cv, epoch: e }, off))
}

fn read_checkpoint(data: &[u8], off: usize) -> Option<(Checkpoint, usize)> {
    let (e, off) = read_u64(data, off)?;
    let (r, off) = read_bytes(data, off)?;
    Some((Checkpoint { epoch: e, root: r }, off))
}

fn read_eth1_data(data: &[u8], off: usize) -> Option<(Eth1Data, usize)> {
    let (dr, off) = read_bytes(data, off)?;
    let (dc, off) = read_u64(data, off)?;
    let (bh, off) = read_bytes(data, off)?;
    Some((Eth1Data { deposit_root: dr, deposit_count: dc, block_hash: bh }, off))
}

fn read_block_header(data: &[u8], off: usize) -> Option<(BeaconBlockHeader, usize)> {
    let (sl, off) = read_u64(data, off)?;
    let (pi, off) = read_u64(data, off)?;
    let (pr, off) = read_bytes(data, off)?;
    let (sr, off) = read_bytes(data, off)?;
    let (br, off) = read_bytes(data, off)?;
    Some((BeaconBlockHeader { slot: sl, proposer_index: pi, parent_root: pr, state_root: sr, body_root: br }, off))
}

fn read_validator(data: &[u8], off: usize) -> Option<(Validator, usize)> {
    let (pk, off) = read_bytes(data, off)?;
    let (wc, off) = read_bytes(data, off)?;
    let (eb, off) = read_u64(data, off)?;
    let (sl, off) = read_bool(data, off)?;
    let (aee, off) = read_u64(data, off)?;
    let (ae, off) = read_u64(data, off)?;
    let (ee, off) = read_u64(data, off)?;
    let (we, off) = read_u64(data, off)?;
    Some((Validator {
        pubkey: pk, withdrawal_credentials: wc, effective_balance: eb, slashed: sl,
        activation_eligibility_epoch: aee, activation_epoch: ae, exit_epoch: ee, withdrawable_epoch: we,
    }, off))
}

fn read_sync_committee(data: &[u8], off: usize) -> Option<(SyncCommittee, usize)> {
    let (pks, off) = read_array(data, off, read_bytes)?;
    let (apk, off) = read_bytes(data, off)?;
    Some((SyncCommittee { pubkeys: pks, aggregate_pubkey: apk }, off))
}

fn read_execution_payload_header(data: &[u8], off: usize) -> Option<(ExecutionPayloadHeader, usize)> {
    let (ph, off) = read_bytes(data, off)?;
    let (fr, off) = read_bytes(data, off)?;
    let (sr, off) = read_bytes(data, off)?;
    let (rr, off) = read_bytes(data, off)?;
    let (lb, off) = read_bytes(data, off)?;
    let (pr, off) = read_bytes(data, off)?;
    let (bn, off) = read_u64(data, off)?;
    let (gl, off) = read_u64(data, off)?;
    let (gu, off) = read_u64(data, off)?;
    let (ts, off) = read_u64(data, off)?;
    let (ed, off) = read_bytes(data, off)?;
    let (bf, off) = read_u64(data, off)?;
    let (bh, off) = read_bytes(data, off)?;
    let (tr, off) = read_bytes(data, off)?;
    let (wr, off) = read_bytes(data, off)?;
    Some((ExecutionPayloadHeader {
        parent_hash: ph, fee_recipient: fr, state_root: sr, receipts_root: rr,
        logs_bloom: lb, prev_randao: pr, block_number: bn, gas_limit: gl,
        gas_used: gu, timestamp: ts, extra_data: ed, base_fee_per_gas: bf,
        block_hash: bh, transactions_root: tr, withdrawals_root: wr,
    }, off))
}

fn read_historical_summary(data: &[u8], off: usize) -> Option<(HistoricalSummary, usize)> {
    let (bsr, off) = read_bytes(data, off)?;
    let (ssr, off) = read_bytes(data, off)?;
    Some((HistoricalSummary { block_summary_root: bsr, state_summary_root: ssr }, off))
}

impl BeaconState {
    pub fn deserialize(data: &[u8]) -> Option<(Self, usize)> {
        let off = 0;
        let (gt, off) = read_u64(data, off)?;
        let (gvr, off) = read_bytes(data, off)?;
        let (sl, off) = read_u64(data, off)?;
        let (fk, off) = read_fork(data, off)?;
        let (lbh, off) = read_block_header(data, off)?;
        let (br, off) = read_array(data, off, read_bytes)?;
        let (sr, off) = read_array(data, off, read_bytes)?;
        let (hr, off) = read_array(data, off, read_bytes)?;
        let (e1d, off) = read_eth1_data(data, off)?;
        let (e1v, off) = read_array(data, off, read_eth1_data)?;
        let (e1i, off) = read_u64(data, off)?;
        let (vals, off) = read_array(data, off, read_validator)?;
        let (bals, off) = read_array(data, off, read_u64)?;
        let (rm, off) = read_array(data, off, read_bytes)?;
        let (sls, off) = read_array(data, off, read_u64)?;
        let (pep, off) = read_array(data, off, read_u8)?;
        let (cep, off) = read_array(data, off, read_u8)?;
        let (jb, off) = read_bytes(data, off)?;
        let (pjc, off) = read_checkpoint(data, off)?;
        let (cjc, off) = read_checkpoint(data, off)?;
        let (fc, off) = read_checkpoint(data, off)?;
        let (is_, off) = read_array(data, off, read_u64)?;
        let (csc, off) = read_sync_committee(data, off)?;
        let (nsc, off) = read_sync_committee(data, off)?;
        let (leph, off) = read_execution_payload_header(data, off)?;
        let (nwi, off) = read_u64(data, off)?;
        let (nwvi, off) = read_u64(data, off)?;
        let (hs, off) = read_array(data, off, read_historical_summary)?;
        Some((BeaconState {
            genesis_time: gt, genesis_validators_root: gvr, slot: sl, fork: fk,
            latest_block_header: lbh, block_roots: br, state_roots: sr, historical_roots: hr,
            eth1_data: e1d, eth1_data_votes: e1v, eth1_deposit_index: e1i,
            validators: vals, balances: bals, randao_mixes: rm, slashings: sls,
            previous_epoch_participation: pep, current_epoch_participation: cep,
            justification_bits: jb, previous_justified_checkpoint: pjc,
            current_justified_checkpoint: cjc, finalized_checkpoint: fc,
            inactivity_scores: is_, current_sync_committee: csc, next_sync_committee: nsc,
            latest_execution_payload_header: leph,
            next_withdrawal_index: nwi, next_withdrawal_validator_index: nwvi,
            historical_summaries: hs,
        }, off))
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        write_u64(&mut buf, self.genesis_time);
        write_bytes(&mut buf, &self.genesis_validators_root);
        write_u64(&mut buf, self.slot);
        // Fork
        write_bytes(&mut buf, &self.fork.previous_version);
        write_bytes(&mut buf, &self.fork.current_version);
        write_u64(&mut buf, self.fork.epoch);
        // Block header
        write_u64(&mut buf, self.latest_block_header.slot);
        write_u64(&mut buf, self.latest_block_header.proposer_index);
        write_bytes(&mut buf, &self.latest_block_header.parent_root);
        write_bytes(&mut buf, &self.latest_block_header.state_root);
        write_bytes(&mut buf, &self.latest_block_header.body_root);
        // Arrays
        write_u32(&mut buf, self.block_roots.len() as u32);
        for r in &self.block_roots { write_bytes(&mut buf, r); }
        write_u32(&mut buf, self.state_roots.len() as u32);
        for r in &self.state_roots { write_bytes(&mut buf, r); }
        write_u32(&mut buf, self.historical_roots.len() as u32);
        for r in &self.historical_roots { write_bytes(&mut buf, r); }
        // Eth1
        write_bytes(&mut buf, &self.eth1_data.deposit_root);
        write_u64(&mut buf, self.eth1_data.deposit_count);
        write_bytes(&mut buf, &self.eth1_data.block_hash);
        write_u32(&mut buf, self.eth1_data_votes.len() as u32);
        for e in &self.eth1_data_votes {
            write_bytes(&mut buf, &e.deposit_root);
            write_u64(&mut buf, e.deposit_count);
            write_bytes(&mut buf, &e.block_hash);
        }
        write_u64(&mut buf, self.eth1_deposit_index);
        // Registry
        write_u32(&mut buf, self.validators.len() as u32);
        for v in &self.validators {
            write_bytes(&mut buf, &v.pubkey);
            write_bytes(&mut buf, &v.withdrawal_credentials);
            write_u64(&mut buf, v.effective_balance);
            write_bool(&mut buf, v.slashed);
            write_u64(&mut buf, v.activation_eligibility_epoch);
            write_u64(&mut buf, v.activation_epoch);
            write_u64(&mut buf, v.exit_epoch);
            write_u64(&mut buf, v.withdrawable_epoch);
        }
        write_u32(&mut buf, self.balances.len() as u32);
        for b in &self.balances { write_u64(&mut buf, *b); }
        // Randomness
        write_u32(&mut buf, self.randao_mixes.len() as u32);
        for r in &self.randao_mixes { write_bytes(&mut buf, r); }
        // Slashings
        write_u32(&mut buf, self.slashings.len() as u32);
        for s in &self.slashings { write_u64(&mut buf, *s); }
        // Participation
        write_u32(&mut buf, self.previous_epoch_participation.len() as u32);
        for p in &self.previous_epoch_participation { write_u8(&mut buf, *p); }
        write_u32(&mut buf, self.current_epoch_participation.len() as u32);
        for p in &self.current_epoch_participation { write_u8(&mut buf, *p); }
        // Finality
        write_bytes(&mut buf, &self.justification_bits);
        write_u64(&mut buf, self.previous_justified_checkpoint.epoch);
        write_bytes(&mut buf, &self.previous_justified_checkpoint.root);
        write_u64(&mut buf, self.current_justified_checkpoint.epoch);
        write_bytes(&mut buf, &self.current_justified_checkpoint.root);
        write_u64(&mut buf, self.finalized_checkpoint.epoch);
        write_bytes(&mut buf, &self.finalized_checkpoint.root);
        // Inactivity
        write_u32(&mut buf, self.inactivity_scores.len() as u32);
        for s in &self.inactivity_scores { write_u64(&mut buf, *s); }
        // Sync committees
        write_u32(&mut buf, self.current_sync_committee.pubkeys.len() as u32);
        for pk in &self.current_sync_committee.pubkeys { write_bytes(&mut buf, pk); }
        write_bytes(&mut buf, &self.current_sync_committee.aggregate_pubkey);
        write_u32(&mut buf, self.next_sync_committee.pubkeys.len() as u32);
        for pk in &self.next_sync_committee.pubkeys { write_bytes(&mut buf, pk); }
        write_bytes(&mut buf, &self.next_sync_committee.aggregate_pubkey);
        // Execution
        write_bytes(&mut buf, &self.latest_execution_payload_header.parent_hash);
        write_bytes(&mut buf, &self.latest_execution_payload_header.fee_recipient);
        write_bytes(&mut buf, &self.latest_execution_payload_header.state_root);
        write_bytes(&mut buf, &self.latest_execution_payload_header.receipts_root);
        write_bytes(&mut buf, &self.latest_execution_payload_header.logs_bloom);
        write_bytes(&mut buf, &self.latest_execution_payload_header.prev_randao);
        write_u64(&mut buf, self.latest_execution_payload_header.block_number);
        write_u64(&mut buf, self.latest_execution_payload_header.gas_limit);
        write_u64(&mut buf, self.latest_execution_payload_header.gas_used);
        write_u64(&mut buf, self.latest_execution_payload_header.timestamp);
        write_bytes(&mut buf, &self.latest_execution_payload_header.extra_data);
        write_u64(&mut buf, self.latest_execution_payload_header.base_fee_per_gas);
        write_bytes(&mut buf, &self.latest_execution_payload_header.block_hash);
        write_bytes(&mut buf, &self.latest_execution_payload_header.transactions_root);
        write_bytes(&mut buf, &self.latest_execution_payload_header.withdrawals_root);
        // Withdrawals
        write_u64(&mut buf, self.next_withdrawal_index);
        write_u64(&mut buf, self.next_withdrawal_validator_index);
        // Historical summaries
        write_u32(&mut buf, self.historical_summaries.len() as u32);
        for hs in &self.historical_summaries {
            write_bytes(&mut buf, &hs.block_summary_root);
            write_bytes(&mut buf, &hs.state_summary_root);
        }
        buf
    }
}

impl SignedBeaconBlock {
    pub fn deserialize(data: &[u8]) -> Option<(Self, usize)> {
        let off = 0;
        let (slot, off) = read_u64(data, off)?;
        let (proposer_index, off) = read_u64(data, off)?;
        let (parent_root, off) = read_bytes(data, off)?;
        let (state_root, off) = read_bytes(data, off)?;
        let (randao_reveal, off) = read_bytes(data, off)?;
        let (eth1_data, off) = read_eth1_data(data, off)?;
        let (graffiti, off) = read_bytes(data, off)?;
        // Simplified: skip operation count, use empty body
        let (_op_count, off) = read_u32(data, off)?;
        let (signature, off) = read_bytes(data, off)?;
        Some((SignedBeaconBlock {
            message: BeaconBlock {
                slot, proposer_index, parent_root, state_root,
                body: BeaconBlockBody {
                    randao_reveal, eth1_data, graffiti,
                    ..Default::default()
                },
            },
            signature,
        }, off))
    }
}
