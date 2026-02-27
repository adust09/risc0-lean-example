use clap::{Parser, ValueEnum};
use methods::{
    GUEST_ETH2_INIT_ELF, GUEST_ETH2_INIT_ID, GUEST_ETH2_NOINIT_ELF, GUEST_ETH2_NOINIT_ID,
    GUEST_RUST_ELF, GUEST_RUST_ETH2_ELF, GUEST_RUST_ETH2_ID, GUEST_RUST_ID, METHOD_ELF,
    METHOD_ID,
};
use risc0_zkvm::{default_executor, default_prover, sha::Digest, ExecutorEnv};
use std::time::Instant;

#[derive(Parser)]
#[command(name = "benchmark", about = "Benchmark Lean vs Rust guest in risc0 zkVM")]
struct Cli {
    /// Benchmark suite: sum (arithmetic) or eth2 (state transition)
    #[arg(long, default_value = "sum")]
    suite: Suite,

    /// Execution mode
    #[arg(long, default_value = "execute")]
    mode: Mode,

    /// Comma-separated input values (N for sum, num_validators for eth2)
    #[arg(long, default_value = "10,100,1000,5000,10000", value_delimiter = ',')]
    inputs: Vec<u32>,

    /// Number of runs for wall-clock measurement
    #[arg(long, default_value_t = 3)]
    runs: usize,

    /// Which guest to benchmark
    #[arg(long, default_value = "both")]
    guest: GuestChoice,
}

#[derive(Clone, ValueEnum)]
enum Suite {
    Sum,
    Eth2,
}

#[derive(Clone, ValueEnum)]
enum Mode {
    Execute,
    Prove,
}

#[derive(Clone, ValueEnum)]
enum GuestChoice {
    Lean,
    Rust,
    Both,
    /// Lean guest without Init library (eth2 only)
    LeanNoinit,
    /// Lean guest with Init library (eth2 only)
    LeanInit,
    /// All eth2 guests (lean-noinit + lean-init + rust)
    All,
}

// ── Common result types ─────────────────────────

struct BenchResult {
    guest_name: &'static str,
    input: u32,
    output: u32,
    user_cycles: u64,
    total_cycles: Option<u64>,
    paging_cycles: Option<u64>,
    segments: usize,
    wall_times_ms: Vec<u128>,
}

impl BenchResult {
    fn median_wall_ms(&self) -> u128 {
        let mut sorted = self.wall_times_ms.clone();
        sorted.sort();
        sorted[sorted.len() / 2]
    }
}

/// Eth2 benchmark result (output is bytes, not u32)
struct Eth2BenchResult {
    guest_name: &'static str,
    num_validators: u32,
    output_bytes: Vec<u8>,
    user_cycles: u64,
    total_cycles: Option<u64>,
    paging_cycles: Option<u64>,
    segments: usize,
    wall_times_ms: Vec<u128>,
}

impl Eth2BenchResult {
    fn median_wall_ms(&self) -> u128 {
        let mut sorted = self.wall_times_ms.clone();
        sorted.sort();
        sorted[sorted.len() / 2]
    }

    /// Check if the execution failed or returned an error marker
    fn is_error(&self) -> bool {
        self.output_bytes.is_empty() || self.output_bytes.len() == 1
    }

    fn error_description(&self) -> &'static str {
        if self.output_bytes.is_empty() {
            return "CRASHED";
        }
        if self.output_bytes.len() != 1 {
            return "OK";
        }
        match self.output_bytes[0] {
            0xFF => "State decode error",
            0xFE => "Block decode error",
            0xFD => "STF error",
            _ => "Unknown error",
        }
    }
}

// ── Sum benchmark functions (existing) ──────────

fn build_env(input: u32) -> ExecutorEnv<'static> {
    ExecutorEnv::builder()
        .write(&input)
        .unwrap()
        .build()
        .unwrap()
}

fn bench_execute(elf: &[u8], input: u32, runs: usize, guest_name: &'static str) -> BenchResult {
    let executor = default_executor();

    let env = build_env(input);
    let start = Instant::now();
    let session = executor.execute(env, elf).unwrap();
    let first_wall = start.elapsed().as_millis();

    let user_cycles = session.cycles();
    let segments = session.segments.len();
    let output: u32 = session.journal.decode().unwrap();

    let mut wall_times = vec![first_wall];
    for _ in 1..runs {
        let env = build_env(input);
        let start = Instant::now();
        let _ = executor.execute(env, elf).unwrap();
        wall_times.push(start.elapsed().as_millis());
    }

    BenchResult {
        guest_name,
        input,
        output,
        user_cycles,
        total_cycles: None,
        paging_cycles: None,
        segments,
        wall_times_ms: wall_times,
    }
}

fn bench_prove(
    elf: &[u8],
    id: impl Into<Digest>,
    input: u32,
    runs: usize,
    guest_name: &'static str,
) -> BenchResult {
    let prover = default_prover();

    let env = build_env(input);
    let start = Instant::now();
    let prove_info = prover.prove(env, elf).unwrap();
    let first_wall = start.elapsed().as_millis();

    let stats = &prove_info.stats;
    let output: u32 = prove_info.receipt.journal.decode().unwrap();

    prove_info
        .receipt
        .verify(id)
        .expect("Receipt verification failed");

    let result = BenchResult {
        guest_name,
        input,
        output,
        user_cycles: stats.user_cycles,
        total_cycles: Some(stats.total_cycles),
        paging_cycles: Some(stats.paging_cycles),
        segments: stats.segments,
        wall_times_ms: vec![first_wall],
    };

    if runs > 1 {
        let mut wall_times = result.wall_times_ms.clone();
        for _ in 1..runs {
            let env = build_env(input);
            let start = Instant::now();
            let _ = prover.prove(env, elf).unwrap();
            wall_times.push(start.elapsed().as_millis());
        }
        return BenchResult {
            wall_times_ms: wall_times,
            ..result
        };
    }

    result
}

// ── Eth2 test data builder ──────────────────────

mod eth2_testdata {
    //! Builds minimal BeaconState + SignedBeaconBlock for testing.
    //! Uses the same binary format as the Lean/Rust guest serializers.

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

    /// Build serialized test input: BeaconState + SignedBeaconBlock
    ///
    /// Creates a minimal but valid state at slot 100 with `num_validators` validators,
    /// and a block for slot 101 (simple 1-slot advance, no epoch boundary).
    pub fn build_test_input(num_validators: usize) -> Vec<u8> {
        let mut buf = Vec::new();

        // ── BeaconState ──

        // genesis_time
        write_u64(&mut buf, 1_000_000);
        // genesis_validators_root
        write_bytes(&mut buf, &zero_bytes(32));
        // slot
        write_u64(&mut buf, 100);

        // fork: previous_version, current_version, epoch
        write_bytes(&mut buf, &[0, 0, 0, 0]);
        write_bytes(&mut buf, &[1, 0, 0, 0]);
        write_u64(&mut buf, 0);

        // latest_block_header: slot, proposer_index, parent_root, state_root, body_root
        write_u64(&mut buf, 100);
        write_u64(&mut buf, 0);
        write_bytes(&mut buf, &zero_bytes(32));
        write_bytes(&mut buf, &zero_bytes(32));
        write_bytes(&mut buf, &zero_bytes(32));

        // block_roots: 200 entries (enough for slot 101 % 8192 = 101)
        let roots_len = 200;
        write_u32(&mut buf, roots_len);
        for _ in 0..roots_len {
            write_bytes(&mut buf, &zero_bytes(32));
        }

        // state_roots: 200 entries
        write_u32(&mut buf, roots_len);
        for _ in 0..roots_len {
            write_bytes(&mut buf, &zero_bytes(32));
        }

        // historical_roots: empty
        write_u32(&mut buf, 0);

        // eth1_data: deposit_root, deposit_count, block_hash
        write_bytes(&mut buf, &zero_bytes(32));
        write_u64(&mut buf, 0);
        write_bytes(&mut buf, &zero_bytes(32));

        // eth1_data_votes: empty
        write_u32(&mut buf, 0);

        // eth1_deposit_index
        write_u64(&mut buf, 0);

        // validators
        let n = num_validators as u32;
        write_u32(&mut buf, n);
        for i in 0..num_validators {
            // pubkey: 48 bytes (unique per validator)
            let mut pk = vec![0u8; 48];
            pk[0] = i as u8;
            pk[1] = (i >> 8) as u8;
            write_bytes(&mut buf, &pk);
            // withdrawal_credentials: 32 bytes
            write_bytes(&mut buf, &zero_bytes(32));
            // effective_balance: 32 ETH
            write_u64(&mut buf, 32_000_000_000);
            // slashed: false
            write_bool(&mut buf, false);
            // activation_eligibility_epoch: 0
            write_u64(&mut buf, 0);
            // activation_epoch: 0
            write_u64(&mut buf, 0);
            // exit_epoch: FAR_FUTURE_EPOCH
            write_u64(&mut buf, FAR_FUTURE_EPOCH);
            // withdrawable_epoch: FAR_FUTURE_EPOCH
            write_u64(&mut buf, FAR_FUTURE_EPOCH);
        }

        // balances: 32 ETH each
        write_u32(&mut buf, n);
        for _ in 0..num_validators {
            write_u64(&mut buf, 32_000_000_000);
        }

        // randao_mixes: 200 entries (enough for epoch 3 % 65536 = 3)
        let mixes_len = 200u32;
        write_u32(&mut buf, mixes_len);
        for _ in 0..mixes_len {
            write_bytes(&mut buf, &zero_bytes(32));
        }

        // slashings: 200 entries
        let slashings_len = 200u32;
        write_u32(&mut buf, slashings_len);
        for _ in 0..slashings_len {
            write_u64(&mut buf, 0);
        }

        // previous_epoch_participation: all flags set (0x07)
        write_u32(&mut buf, n);
        for _ in 0..num_validators {
            write_u8(&mut buf, 0x07);
        }

        // current_epoch_participation: all flags set
        write_u32(&mut buf, n);
        for _ in 0..num_validators {
            write_u8(&mut buf, 0x07);
        }

        // justification_bits: 4 bytes
        write_bytes(&mut buf, &[0, 0, 0, 0]);

        // previous_justified_checkpoint: epoch + root
        write_u64(&mut buf, 0);
        write_bytes(&mut buf, &zero_bytes(32));

        // current_justified_checkpoint
        write_u64(&mut buf, 0);
        write_bytes(&mut buf, &zero_bytes(32));

        // finalized_checkpoint
        write_u64(&mut buf, 0);
        write_bytes(&mut buf, &zero_bytes(32));

        // inactivity_scores
        write_u32(&mut buf, n);
        for _ in 0..num_validators {
            write_u64(&mut buf, 0);
        }

        // current_sync_committee: 512 pubkeys + aggregate_pubkey
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

        // latest_execution_payload_header (15 fields)
        write_bytes(&mut buf, &zero_bytes(32)); // parent_hash
        write_bytes(&mut buf, &zero_bytes(20)); // fee_recipient
        write_bytes(&mut buf, &zero_bytes(32)); // state_root
        write_bytes(&mut buf, &zero_bytes(32)); // receipts_root
        write_bytes(&mut buf, &zero_bytes(256)); // logs_bloom
        write_bytes(&mut buf, &zero_bytes(32)); // prev_randao
        write_u64(&mut buf, 0); // block_number
        write_u64(&mut buf, 0); // gas_limit
        write_u64(&mut buf, 0); // gas_used
        write_u64(&mut buf, 0); // timestamp
        write_bytes(&mut buf, &[]); // extra_data (empty)
        write_u64(&mut buf, 0); // base_fee_per_gas
        write_bytes(&mut buf, &zero_bytes(32)); // block_hash
        write_bytes(&mut buf, &zero_bytes(32)); // transactions_root
        write_bytes(&mut buf, &zero_bytes(32)); // withdrawals_root

        // next_withdrawal_index
        write_u64(&mut buf, 0);
        // next_withdrawal_validator_index
        write_u64(&mut buf, 0);

        // historical_summaries: empty
        write_u32(&mut buf, 0);

        // ── SignedBeaconBlock ──

        // slot: 101 (one slot ahead of state.slot=100)
        write_u64(&mut buf, 101);
        // proposer_index: must match getBeaconProposerIndex stub (slot % validator_count)
        // slot=101, validators=N, so proposer_index = 101 % N
        write_u64(&mut buf, 101 % num_validators as u64);
        // parent_root
        write_bytes(&mut buf, &zero_bytes(32));
        // state_root
        write_bytes(&mut buf, &zero_bytes(32));
        // randao_reveal
        write_bytes(&mut buf, &zero_bytes(96));
        // eth1_data: deposit_root, deposit_count, block_hash
        write_bytes(&mut buf, &zero_bytes(32));
        write_u64(&mut buf, 0);
        write_bytes(&mut buf, &zero_bytes(32));
        // graffiti
        write_bytes(&mut buf, &zero_bytes(32));
        // op_count (0 = no operations)
        write_u32(&mut buf, 0);
        // signature
        write_bytes(&mut buf, &zero_bytes(96));

        buf
    }
}

// ── Eth2 benchmark functions ────────────────────

fn build_eth2_env(test_input: &[u8]) -> ExecutorEnv<'static> {
    let input_vec = test_input.to_vec();
    ExecutorEnv::builder()
        .write(&input_vec)
        .unwrap()
        .build()
        .unwrap()
}

fn bench_eth2_execute(
    elf: &[u8],
    test_input: &[u8],
    num_validators: u32,
    runs: usize,
    guest_name: &'static str,
) -> Eth2BenchResult {
    let executor = default_executor();

    let env = build_eth2_env(test_input);
    let start = Instant::now();
    let result = executor.execute(env, elf);
    let first_wall = start.elapsed().as_millis();

    match result {
        Ok(session) => {
            let user_cycles = session.cycles();
            let segments = session.segments.len();
            let output_bytes = session.journal.bytes.clone();


            let mut wall_times = vec![first_wall];
            for _ in 1..runs {
                let env = build_eth2_env(test_input);
                let start = Instant::now();
                let _ = executor.execute(env, elf);
                wall_times.push(start.elapsed().as_millis());
            }

            Eth2BenchResult {
                guest_name,
                num_validators,
                output_bytes,
                user_cycles,
                total_cycles: None,
                paging_cycles: None,
                segments,
                wall_times_ms: wall_times,
            }
        }
        Err(e) => {
            eprintln!("  {} FAILED: {}", guest_name, e);
            Eth2BenchResult {
                guest_name,
                num_validators,
                output_bytes: vec![],
                user_cycles: 0,
                total_cycles: None,
                paging_cycles: None,
                segments: 0,
                wall_times_ms: vec![first_wall],
            }
        }
    }
}

fn bench_eth2_prove(
    elf: &[u8],
    id: impl Into<Digest>,
    test_input: &[u8],
    num_validators: u32,
    runs: usize,
    guest_name: &'static str,
) -> Eth2BenchResult {
    let prover = default_prover();
    let digest = id.into();

    let env = build_eth2_env(test_input);
    let start = Instant::now();
    let result = prover.prove(env, elf);
    let first_wall = start.elapsed().as_millis();

    match result {
        Ok(prove_info) => {
            let stats = &prove_info.stats;
            let output_bytes = prove_info.receipt.journal.bytes.clone();

            if let Err(e) = prove_info.receipt.verify(digest) {
                eprintln!("  {} receipt verification failed: {}", guest_name, e);
            }

            let result = Eth2BenchResult {
                guest_name,
                num_validators,
                output_bytes,
                user_cycles: stats.user_cycles,
                total_cycles: Some(stats.total_cycles),
                paging_cycles: Some(stats.paging_cycles),
                segments: stats.segments,
                wall_times_ms: vec![first_wall],
            };

            if runs > 1 {
                let mut wall_times = result.wall_times_ms.clone();
                for _ in 1..runs {
                    let env = build_eth2_env(test_input);
                    let start = Instant::now();
                    let _ = prover.prove(env, elf);
                    wall_times.push(start.elapsed().as_millis());
                }
                return Eth2BenchResult {
                    wall_times_ms: wall_times,
                    ..result
                };
            }

            result
        }
        Err(e) => {
            eprintln!("  {} PROVE FAILED: {}", guest_name, e);
            Eth2BenchResult {
                guest_name,
                num_validators,
                output_bytes: vec![],
                user_cycles: 0,
                total_cycles: None,
                paging_cycles: None,
                segments: 0,
                wall_times_ms: vec![first_wall],
            }
        }
    }
}

// ── Formatting helpers ──────────────────────────

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn format_time(ms: u128) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.1}s", ms as f64 / 1000.0)
    }
}

fn print_header(mode: &Mode) {
    match mode {
        Mode::Execute => {
            println!(
                "{:<14} {:>7} {:>14} {:>10} {:>10}",
                "Guest", "N", "User Cycles", "Segments", "Time"
            );
            println!("{}", "-".repeat(61));
        }
        Mode::Prove => {
            println!(
                "{:<14} {:>7} {:>14} {:>14} {:>14} {:>10} {:>10}",
                "Guest", "N", "User Cycles", "Total Cycles", "Paging Cycles", "Segments", "Time"
            );
            println!("{}", "-".repeat(91));
        }
    }
}

fn print_result(result: &BenchResult, mode: &Mode) {
    match mode {
        Mode::Execute => {
            println!(
                "{:<14} {:>7} {:>14} {:>10} {:>10}",
                result.guest_name,
                result.input,
                format_number(result.user_cycles),
                result.segments,
                format_time(result.median_wall_ms()),
            );
        }
        Mode::Prove => {
            println!(
                "{:<14} {:>7} {:>14} {:>14} {:>14} {:>10} {:>10}",
                result.guest_name,
                result.input,
                format_number(result.user_cycles),
                format_number(result.total_cycles.unwrap_or(0)),
                format_number(result.paging_cycles.unwrap_or(0)),
                result.segments,
                format_time(result.median_wall_ms()),
            );
        }
    }
}

fn print_eth2_result(result: &Eth2BenchResult, mode: &Mode) {
    let status = if result.is_error() {
        format!("ERR:{}", result.error_description())
    } else {
        format!("{}B", format_number(result.output_bytes.len() as u64))
    };

    match mode {
        Mode::Execute => {
            println!(
                "{:<14} {:>7} {:>14} {:>10} {:>10}  [{}]",
                result.guest_name,
                result.num_validators,
                format_number(result.user_cycles),
                result.segments,
                format_time(result.median_wall_ms()),
                status,
            );
        }
        Mode::Prove => {
            println!(
                "{:<14} {:>7} {:>14} {:>14} {:>14} {:>10} {:>10}  [{}]",
                result.guest_name,
                result.num_validators,
                format_number(result.user_cycles),
                format_number(result.total_cycles.unwrap_or(0)),
                format_number(result.paging_cycles.unwrap_or(0)),
                result.segments,
                format_time(result.median_wall_ms()),
                status,
            );
        }
    }
}

fn print_ratio(lean: &BenchResult, rust: &BenchResult, mode: &Mode) {
    let cycle_ratio = if rust.user_cycles > 0 {
        format!("{:.1}x", lean.user_cycles as f64 / rust.user_cycles as f64)
    } else {
        "N/A".to_string()
    };
    let seg_ratio = if rust.segments > 0 {
        format!("{:.1}x", lean.segments as f64 / rust.segments as f64)
    } else {
        "N/A".to_string()
    };
    let time_ratio = if rust.median_wall_ms() > 0 {
        format!(
            "{:.1}x",
            lean.median_wall_ms() as f64 / rust.median_wall_ms() as f64
        )
    } else {
        "N/A".to_string()
    };

    match mode {
        Mode::Execute => {
            println!(
                "{:<14} {:>7} {:>14} {:>10} {:>10}",
                "Ratio", lean.input, cycle_ratio, seg_ratio, time_ratio,
            );
        }
        Mode::Prove => {
            let total_ratio = match (lean.total_cycles, rust.total_cycles) {
                (Some(l), Some(r)) if r > 0 => format!("{:.1}x", l as f64 / r as f64),
                _ => "N/A".to_string(),
            };
            let paging_ratio = match (lean.paging_cycles, rust.paging_cycles) {
                (Some(l), Some(r)) if r > 0 => format!("{:.1}x", l as f64 / r as f64),
                _ => "N/A".to_string(),
            };
            println!(
                "{:<14} {:>7} {:>14} {:>14} {:>14} {:>10} {:>10}",
                "Ratio", lean.input, cycle_ratio, total_ratio, paging_ratio, seg_ratio, time_ratio,
            );
        }
    }
}

fn print_eth2_ratio(lean: &Eth2BenchResult, rust: &Eth2BenchResult, mode: &Mode) {
    let cycle_ratio = if rust.user_cycles > 0 {
        format!("{:.1}x", lean.user_cycles as f64 / rust.user_cycles as f64)
    } else {
        "N/A".to_string()
    };
    let seg_ratio = if rust.segments > 0 {
        format!("{:.1}x", lean.segments as f64 / rust.segments as f64)
    } else {
        "N/A".to_string()
    };

    match mode {
        Mode::Execute => {
            println!(
                "{:<14} {:>7} {:>14} {:>10}",
                "  Ratio(L/R)", lean.num_validators, cycle_ratio, seg_ratio,
            );
        }
        Mode::Prove => {
            let total_ratio = match (lean.total_cycles, rust.total_cycles) {
                (Some(l), Some(r)) if r > 0 => format!("{:.1}x", l as f64 / r as f64),
                _ => "N/A".to_string(),
            };
            println!(
                "{:<14} {:>7} {:>14} {:>14} {:>10}",
                "  Ratio(L/R)", lean.num_validators, cycle_ratio, total_ratio, seg_ratio,
            );
        }
    }
}

// ── Suite runners ───────────────────────────────

fn run_sum_benchmark(cli: &Cli) {
    let run_lean = matches!(cli.guest, GuestChoice::Lean | GuestChoice::Both);
    let run_rust = matches!(cli.guest, GuestChoice::Rust | GuestChoice::Both);

    if run_lean {
        println!(
            "Lean ELF size: {} bytes",
            format_number(METHOD_ELF.len() as u64)
        );
    }
    if run_rust {
        println!(
            "Rust ELF size: {} bytes",
            format_number(GUEST_RUST_ELF.len() as u64)
        );
    }
    if run_lean && run_rust {
        let ratio = METHOD_ELF.len() as f64 / GUEST_RUST_ELF.len() as f64;
        println!("ELF size ratio (Lean/Rust): {:.1}x", ratio);
    }
    println!();

    print_header(&cli.mode);

    for &input in &cli.inputs {
        let lean_result = if run_lean {
            Some(match cli.mode {
                Mode::Execute => bench_execute(METHOD_ELF, input, cli.runs, "Lean"),
                Mode::Prove => bench_prove(METHOD_ELF, METHOD_ID, input, cli.runs, "Lean"),
            })
        } else {
            None
        };

        let rust_result = if run_rust {
            Some(match cli.mode {
                Mode::Execute => bench_execute(GUEST_RUST_ELF, input, cli.runs, "Rust"),
                Mode::Prove => {
                    bench_prove(GUEST_RUST_ELF, GUEST_RUST_ID, input, cli.runs, "Rust")
                }
            })
        } else {
            None
        };

        if let (Some(ref lean), Some(ref rust)) = (&lean_result, &rust_result) {
            assert_eq!(
                lean.output, rust.output,
                "Output mismatch for N={}: Lean={}, Rust={}",
                input, lean.output, rust.output
            );
        }

        if let Some(ref r) = lean_result {
            print_result(r, &cli.mode);
        }
        if let Some(ref r) = rust_result {
            print_result(r, &cli.mode);
        }
        if let (Some(ref lean), Some(ref rust)) = (&lean_result, &rust_result) {
            print_ratio(lean, rust, &cli.mode);
        }

        if cli.inputs.last() != Some(&input) {
            println!();
        }
    }
}

fn run_eth2_benchmark(cli: &Cli) {
    let run_noinit = matches!(
        cli.guest,
        GuestChoice::LeanNoinit | GuestChoice::All | GuestChoice::Both | GuestChoice::Lean
    );
    let run_init = matches!(
        cli.guest,
        GuestChoice::LeanInit | GuestChoice::All | GuestChoice::Both
    );
    let run_rust = matches!(
        cli.guest,
        GuestChoice::Rust | GuestChoice::All | GuestChoice::Both
    );

    println!("=== ETH2 State Transition Benchmark ===");
    println!("State: slot 100 -> 101 (no epoch boundary)");
    println!();

    // Print ELF sizes
    if run_noinit {
        println!(
            "Lean (no-init) ELF size: {} bytes",
            format_number(GUEST_ETH2_NOINIT_ELF.len() as u64)
        );
    }
    if run_init {
        println!(
            "Lean (init)    ELF size: {} bytes",
            format_number(GUEST_ETH2_INIT_ELF.len() as u64)
        );
    }
    if run_rust {
        println!(
            "Rust           ELF size: {} bytes",
            format_number(GUEST_RUST_ETH2_ELF.len() as u64)
        );
    }
    println!();

    print_header(&cli.mode);

    for &num_val in &cli.inputs {
        let test_input = eth2_testdata::build_test_input(num_val as usize);
        println!(
            "  [input: {} validators, {} bytes serialized]",
            num_val,
            format_number(test_input.len() as u64)
        );

        let noinit_result = if run_noinit {
            Some(match cli.mode {
                Mode::Execute => bench_eth2_execute(
                    GUEST_ETH2_NOINIT_ELF,
                    &test_input,
                    num_val,
                    cli.runs,
                    "Lean(no-init)",
                ),
                Mode::Prove => bench_eth2_prove(
                    GUEST_ETH2_NOINIT_ELF,
                    GUEST_ETH2_NOINIT_ID,
                    &test_input,
                    num_val,
                    cli.runs,
                    "Lean(no-init)",
                ),
            })
        } else {
            None
        };

        let init_result = if run_init {
            Some(match cli.mode {
                Mode::Execute => bench_eth2_execute(
                    GUEST_ETH2_INIT_ELF,
                    &test_input,
                    num_val,
                    cli.runs,
                    "Lean(init)",
                ),
                Mode::Prove => bench_eth2_prove(
                    GUEST_ETH2_INIT_ELF,
                    GUEST_ETH2_INIT_ID,
                    &test_input,
                    num_val,
                    cli.runs,
                    "Lean(init)",
                ),
            })
        } else {
            None
        };

        let rust_result = if run_rust {
            Some(match cli.mode {
                Mode::Execute => bench_eth2_execute(
                    GUEST_RUST_ETH2_ELF,
                    &test_input,
                    num_val,
                    cli.runs,
                    "Rust",
                ),
                Mode::Prove => bench_eth2_prove(
                    GUEST_RUST_ETH2_ELF,
                    GUEST_RUST_ETH2_ID,
                    &test_input,
                    num_val,
                    cli.runs,
                    "Rust",
                ),
            })
        } else {
            None
        };

        // Print results
        if let Some(ref r) = noinit_result {
            print_eth2_result(r, &cli.mode);
        }
        if let Some(ref r) = init_result {
            print_eth2_result(r, &cli.mode);
        }
        if let Some(ref r) = rust_result {
            print_eth2_result(r, &cli.mode);
        }

        // Compare outputs between guests
        let all_results: Vec<&Eth2BenchResult> = [&noinit_result, &init_result, &rust_result]
            .iter()
            .filter_map(|r| r.as_ref())
            .collect();

        if all_results.len() >= 2 {
            let first = &all_results[0].output_bytes;
            for r in &all_results[1..] {
                if r.output_bytes != *first {
                    println!(
                        "  WARNING: output mismatch between {} and {}!",
                        all_results[0].guest_name, r.guest_name
                    );
                    println!(
                        "    {} -> {} ({} bytes)",
                        all_results[0].guest_name,
                        all_results[0].error_description(),
                        all_results[0].output_bytes.len()
                    );
                    println!(
                        "    {} -> {} ({} bytes)",
                        r.guest_name,
                        r.error_description(),
                        r.output_bytes.len()
                    );
                } else {
                    println!(
                        "  OK: {} == {} ({} bytes)",
                        all_results[0].guest_name,
                        r.guest_name,
                        first.len()
                    );
                }
            }
        }

        // Print Lean/Rust ratios if we have both
        if let (Some(ref lean), Some(ref rust)) = (&init_result, &rust_result) {
            print_eth2_ratio(lean, rust, &cli.mode);
        } else if let (Some(ref lean), Some(ref rust)) = (&noinit_result, &rust_result) {
            print_eth2_ratio(lean, rust, &cli.mode);
        }

        if cli.inputs.last() != Some(&num_val) {
            println!();
        }
    }
}

// ── Main ────────────────────────────────────────

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.suite {
        Suite::Sum => run_sum_benchmark(&cli),
        Suite::Eth2 => run_eth2_benchmark(&cli),
    }
}
