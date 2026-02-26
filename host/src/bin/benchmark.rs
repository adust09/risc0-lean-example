use clap::{Parser, ValueEnum};
use methods::{GUEST_RUST_ELF, GUEST_RUST_ID, METHOD_ELF, METHOD_ID};
use risc0_zkvm::{default_executor, default_prover, sha::Digest, ExecutorEnv};
use std::time::Instant;

#[derive(Parser)]
#[command(name = "benchmark", about = "Benchmark Lean vs Rust guest in risc0 zkVM")]
struct Cli {
    /// Execution mode
    #[arg(long, default_value = "execute")]
    mode: Mode,

    /// Comma-separated input values
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
enum Mode {
    Execute,
    Prove,
}

#[derive(Clone, ValueEnum)]
enum GuestChoice {
    Lean,
    Rust,
    Both,
}

/// Per-run result for a single (guest, input) pair
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

fn build_env(input: u32) -> ExecutorEnv<'static> {
    ExecutorEnv::builder()
        .write(&input)
        .unwrap()
        .build()
        .unwrap()
}

fn bench_execute(elf: &[u8], input: u32, runs: usize, guest_name: &'static str) -> BenchResult {
    let executor = default_executor();

    // First run to capture cycle counts (deterministic, so one run is enough)
    let env = build_env(input);
    let start = Instant::now();
    let session = executor.execute(env, elf).unwrap();
    let first_wall = start.elapsed().as_millis();

    let user_cycles = session.cycles();
    let segments = session.segments.len();
    let output: u32 = session.journal.decode().unwrap();

    let mut wall_times = vec![first_wall];

    // Additional runs for wall-clock variance
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

fn bench_prove(elf: &[u8], id: impl Into<Digest>, input: u32, runs: usize, guest_name: &'static str) -> BenchResult {
    let prover = default_prover();

    // First run to capture stats
    let env = build_env(input);
    let start = Instant::now();
    let prove_info = prover.prove(env, elf).unwrap();
    let first_wall = start.elapsed().as_millis();

    let stats = &prove_info.stats;
    let output: u32 = prove_info.receipt.journal.decode().unwrap();

    // Verify the receipt
    prove_info.receipt.verify(id).expect("Receipt verification failed");

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

    // Additional runs for wall-clock variance (prove is expensive, so optional)
    if runs > 1 {
        let mut wall_times = result.wall_times_ms.clone();
        for _ in 1..(runs) {
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
                "{:<8} {:>7} {:>14} {:>10} {:>10}",
                "Guest", "N", "User Cycles", "Segments", "Time"
            );
            println!("{}", "-".repeat(55));
        }
        Mode::Prove => {
            println!(
                "{:<8} {:>7} {:>14} {:>14} {:>14} {:>10} {:>10}",
                "Guest", "N", "User Cycles", "Total Cycles", "Paging Cycles", "Segments", "Time"
            );
            println!("{}", "-".repeat(85));
        }
    }
}

fn print_result(result: &BenchResult, mode: &Mode) {
    match mode {
        Mode::Execute => {
            println!(
                "{:<8} {:>7} {:>14} {:>10} {:>10}",
                result.guest_name,
                result.input,
                format_number(result.user_cycles),
                result.segments,
                format_time(result.median_wall_ms()),
            );
        }
        Mode::Prove => {
            println!(
                "{:<8} {:>7} {:>14} {:>14} {:>14} {:>10} {:>10}",
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
        format!("{:.1}x", lean.median_wall_ms() as f64 / rust.median_wall_ms() as f64)
    } else {
        "N/A".to_string()
    };

    match mode {
        Mode::Execute => {
            println!(
                "{:<8} {:>7} {:>14} {:>10} {:>10}",
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
                "{:<8} {:>7} {:>14} {:>14} {:>14} {:>10} {:>10}",
                "Ratio", lean.input, cycle_ratio, total_ratio, paging_ratio, seg_ratio, time_ratio,
            );
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let run_lean = matches!(cli.guest, GuestChoice::Lean | GuestChoice::Both);
    let run_rust = matches!(cli.guest, GuestChoice::Rust | GuestChoice::Both);

    // Print ELF sizes
    if run_lean {
        println!("Lean ELF size: {} bytes", format_number(METHOD_ELF.len() as u64));
    }
    if run_rust {
        println!("Rust ELF size: {} bytes", format_number(GUEST_RUST_ELF.len() as u64));
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
                Mode::Prove => bench_prove(GUEST_RUST_ELF, GUEST_RUST_ID, input, cli.runs, "Rust"),
            })
        } else {
            None
        };

        // Verify outputs match
        if let (Some(ref lean), Some(ref rust)) = (&lean_result, &rust_result) {
            assert_eq!(
                lean.output, rust.output,
                "Output mismatch for N={}: Lean={}, Rust={}",
                input, lean.output, rust.output
            );
        }

        // Print results
        if let Some(ref r) = lean_result {
            print_result(r, &cli.mode);
        }
        if let Some(ref r) = rust_result {
            print_result(r, &cli.mode);
        }
        if let (Some(ref lean), Some(ref rust)) = (&lean_result, &rust_result) {
            print_ratio(lean, rust, &cli.mode);
        }

        // Separator between input groups
        if cli.inputs.last() != Some(&input) {
            println!();
        }
    }
}
