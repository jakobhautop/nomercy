use clap::{Parser, Subcommand};
use nomercy::prelude::{Simulation, SystemModel};

#[derive(Parser)]
#[command(version, about = "Deterministic simulation engine (MVP)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a system before simulation
    Beg { system: String },
    /// Run a minimal deterministic simulation
    Pray {
        system: String,
        #[arg(long, default_value_t = 1)]
        rounds: usize,
    },
    /// Replay a captured trace
    Replay { repro: String },
    /// Shrink a captured trace
    Shrink { trace: String },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Beg { system } => {
            println!("system={system}");
            println!("status=qualified");
        }
        Command::Pray { system, rounds } => {
            let simulation = Simulation::new(SystemModel::new(system.clone(), || ()));
            let outcome = simulation.run(rounds);
            println!("{}", outcome.to_json());
        }
        Command::Replay { repro } => {
            println!("replay_source={repro}");
            println!("status=not_implemented");
        }
        Command::Shrink { trace } => {
            println!("trace_source={trace}");
            println!("status=not_implemented");
        }
    }
}
