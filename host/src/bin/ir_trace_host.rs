use std::process::Command;
use std::time::Instant;

use clap::{Parser, ValueEnum};
use sha2::{Digest, Sha256};

use methods::{GUEST_IR_TRACE_ELF, GUEST_IR_TRACE_ID};
use risc0_zkvm::{default_executor, default_prover, ExecutorEnv};

#[derive(Clone, ValueEnum)]
enum Mode {
    Execute,
    Prove,
}

#[derive(Parser)]
#[command(name = "ir-trace-host", about = "Host driver for IR trace verification")]
struct Args {
    #[arg(long, help = "Path to IR program JSON")]
    ir: String,

    #[arg(long, help = "Path to input data file")]
    input: String,

    #[arg(long, default_value = "risc0_main_eth2", help = "Entry point function name")]
    entry: String,

    #[arg(long, default_value = "trace.bin", help = "Path to trace file (intermediate)")]
    trace: String,

    #[arg(long, help = "Pass input as a u32 scalar instead of ByteArray")]
    scalar_input: Option<u32>,

    #[arg(long, default_value = "execute", help = "Execution mode: execute (cycle count only) or prove")]
    mode: Mode,

    #[arg(long, default_value_t = 1, help = "Number of runs for wall-time measurement")]
    runs: usize,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    // Read IR program and compute hash
    let ir_program = std::fs::read(&args.ir).expect("Failed to read IR program");
    let ir_hash = sha256(&ir_program);

    // Read input data
    let input = std::fs::read(&args.input).expect("Failed to read input file");

    println!("IR program: {} bytes, hash: {}", ir_program.len(), hex(&ir_hash));
    println!("Input: {} bytes", input.len());

    // Step 1: Generate trace using ir-trace tool
    println!("Generating execution trace...");
    let mut trace_args = vec![
        "run".to_string(),
        "-p".to_string(),
        "ir-trace".to_string(),
        "--bin".to_string(),
        "ir-trace".to_string(),
        "--".to_string(),
        "--ir".to_string(),
        args.ir.clone(),
        "--input".to_string(),
        args.input.clone(),
        "--output".to_string(),
        args.trace.clone(),
        "--entry".to_string(),
        args.entry.clone(),
    ];
    if let Some(scalar) = args.scalar_input {
        trace_args.push("--scalar-input".to_string());
        trace_args.push(scalar.to_string());
    }
    let trace_status = Command::new("cargo")
        .args(&trace_args)
        .status()
        .expect("Failed to run ir-trace tool");

    if !trace_status.success() {
        panic!("ir-trace tool failed with exit code: {:?}", trace_status.code());
    }

    let trace_data = std::fs::read(&args.trace).expect("Failed to read generated trace");
    println!("Trace generated: {} bytes (bincode)", trace_data.len());

    match args.mode {
        Mode::Execute => run_execute(&args, &ir_hash, &input, &trace_data),
        Mode::Prove => run_prove(&ir_hash, &input, &trace_data),
    }
}

fn build_env(ir_hash: &[u8; 32], input: &[u8], trace_data: &[u8]) -> ExecutorEnv<'static> {
    ExecutorEnv::builder()
        .write(ir_hash)
        .unwrap()
        .write(&input.to_vec())
        .unwrap()
        .write(&trace_data.to_vec())
        .unwrap()
        .build()
        .unwrap()
}

fn run_execute(args: &Args, ir_hash: &[u8; 32], input: &[u8], trace_data: &[u8]) {
    println!("\nRunning zkVM executor (cycle count, no proving)...");
    let executor = default_executor();

    let env = build_env(ir_hash, input, trace_data);
    let start = Instant::now();
    let session = executor.execute(env, GUEST_IR_TRACE_ELF).unwrap();
    let first_wall = start.elapsed();

    let user_cycles = session.cycles();
    let segments = session.segments.len();
    let output: Vec<u8> = session.journal.bytes.clone();

    let mut wall_times = vec![first_wall];
    for _ in 1..args.runs {
        let env = build_env(ir_hash, input, trace_data);
        let start = Instant::now();
        let _ = executor.execute(env, GUEST_IR_TRACE_ELF).unwrap();
        wall_times.push(start.elapsed());
    }

    wall_times.sort();
    let median = wall_times[wall_times.len() / 2];

    println!();
    println!("=== IR Trace zkVM Benchmark ===");
    println!("Mode: execute");
    println!("Trace: {} bytes (bincode)", format_number(trace_data.len() as u64));
    println!();
    println!("User Cycles:    {}", format_number(user_cycles));
    println!("Segments:       {}", segments);
    if args.runs > 1 {
        println!("Wall Time:      {} (median of {} runs)", format_duration(median), args.runs);
        println!("  Min:          {}", format_duration(wall_times[0]));
        println!("  Max:          {}", format_duration(wall_times[wall_times.len() - 1]));
    } else {
        println!("Wall Time:      {}", format_duration(median));
    }

    print_output_summary(&output);
}

fn run_prove(ir_hash: &[u8; 32], input: &[u8], trace_data: &[u8]) {
    println!("\nRunning zkVM prover...");
    let env = build_env(ir_hash, input, trace_data);

    let start = Instant::now();
    let prove_info = default_prover()
        .prove(env, GUEST_IR_TRACE_ELF)
        .unwrap();
    let wall_time = start.elapsed();

    let receipt = &prove_info.receipt;
    receipt.verify(GUEST_IR_TRACE_ID).expect("Receipt verification failed");

    let stats = &prove_info.stats;
    let output: Vec<u8> = receipt.journal.bytes.clone();

    println!();
    println!("=== IR Trace zkVM Benchmark ===");
    println!("Mode: prove");
    println!("Trace: {} bytes (bincode)", format_number(trace_data.len() as u64));
    println!();
    println!("User Cycles:    {}", format_number(stats.user_cycles));
    println!("Total Cycles:   {}", format_number(stats.total_cycles));
    println!("Paging Cycles:  {}", format_number(stats.paging_cycles));
    println!("Segments:       {}", stats.segments);
    println!("Wall Time:      {}", format_duration(wall_time));

    print_output_summary(&output);
}

fn print_output_summary(output: &[u8]) {
    println!();
    println!("=== Output ===");
    println!("Size: {} bytes", output.len());
    if !output.is_empty() {
        match output[0] {
            0xFF => println!("Status: State decode error"),
            0xFE => println!("Status: Block decode error"),
            0xFD => {
                let msg = String::from_utf8_lossy(&output[1..]);
                println!("Status: STF error: {}", msg);
            }
            _ => println!("Status: Success (first byte: 0x{:02X})", output[0]),
        }
    }
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{},{:03},{:03}", n / 1_000_000, (n / 1_000) % 1_000, n % 1_000)
    } else if n >= 1_000 {
        format!("{},{:03}", n / 1_000, n % 1_000)
    } else {
        format!("{}", n)
    }
}

fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs_f64();
    if secs >= 1.0 {
        format!("{:.2}s", secs)
    } else if secs >= 0.001 {
        format!("{:.2}ms", secs * 1000.0)
    } else {
        format!("{:.0}us", secs * 1_000_000.0)
    }
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
