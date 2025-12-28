use clap::{Parser, Subcommand};
use flake::{run_flake_schedule, FlakeOp};
use nomercy_core::EngineConfig;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "nomercy",
    about = "Deterministic adversarial simulation engine (early scaffold)",
    version
)]
struct Cli {
    /// Enable verbose output for debugging early development flows.
    #[arg(short, long, global = true)]
    verbose: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Perform determinism qualification.
    Beg {
        /// Target system name. Defaults to the built-in flake mock.
        system: Option<String>,
    },
    /// Run a simulation schedule.
    Pray {
        /// Target system name. Defaults to the built-in flake mock.
        system: Option<String>,
    },
    /// Replay a previously captured repro.
    Replay {
        /// Path to repro JSON.
        repro: Option<PathBuf>,
    },
    /// Shrink a trace to a minimal reproduction.
    Shrink {
        /// Path to trace JSON.
        trace: Option<PathBuf>,
    },
    /// Explore a system interactively (placeholder).
    Explore {
        /// Target system name.
        system: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Beg { system } => handle_beg(system),
        Commands::Pray { system } => handle_pray(system),
        Commands::Replay { repro } => handle_replay(repro),
        Commands::Shrink { trace } => handle_shrink(trace),
        Commands::Explore { system } => handle_explore(system),
    }
}

fn handle_beg(system: Option<String>) {
    let target = system.unwrap_or_else(|| "flake".to_string());
    println!("seed=0");
    println!("adapter={}", target);
    println!("status=qualification_stub");
}

fn handle_pray(system: Option<String>) {
    let target = system.unwrap_or_else(|| "flake".to_string());
    if target == "flake" {
        run_flake_demo();
        return;
    }

    println!("seed=0");
    println!("adapter={}", target);
    println!("status=pray_stub");
}

fn handle_replay(repro: Option<PathBuf>) {
    println!(
        "repro={}",
        repro
            .unwrap_or_else(|| PathBuf::from("repro.json"))
            .display()
    );
    println!("status=replay_stub");
}

fn handle_shrink(trace: Option<PathBuf>) {
    println!(
        "trace={}",
        trace
            .unwrap_or_else(|| PathBuf::from("trace.json"))
            .display()
    );
    println!("status=shrink_stub");
}

fn handle_explore(system: Option<String>) {
    let target = system.unwrap_or_else(|| "flake".to_string());
    println!("adapter={}", target);
    println!("status=explore_stub");
}

fn run_flake_demo() {
    let operations = vec![
        FlakeOp::Increment(1),
        FlakeOp::Increment(2),
        FlakeOp::Decrement(1),
    ];
    let outcome = run_flake_schedule(1, Some(8), &operations);
    let EngineConfig {
        seed,
        budget,
        system_config: _,
    } = outcome.config;

    println!("seed={seed}");
    if let Some(budget) = budget {
        println!("config:");
        println!("  budget={budget}");
    }

    for step in outcome.steps {
        println!(
            "step={} op={:?} counter={}",
            step.index, step.operation, step.observation.counter
        );
    }

    println!(
        "crash_state={{counter: {}, journal_len: {}}}",
        outcome.crash_state.counter,
        outcome.crash_state.journal.len()
    );
    println!("status=ok");
}
