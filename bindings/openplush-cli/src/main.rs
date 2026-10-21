//! `openplush-cli`: every engine capability as file-in, file-out.

mod commands;
mod failure;
mod input;
mod obj;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use openplush_pipeline::LoopRef;

use crate::commands::{repair, validate};
use crate::failure::Failure;

/// Turn a 3D model into a printable plush sewing pattern.
#[derive(Debug, Parser)]
#[command(name = "openplush-cli", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Report a model's defects without changing it.
    Validate(Validate),
    /// Repair a model, every step on unless turned off, and write it as OBJ.
    Repair(Repair),
    /// Flatten a model along a seam set into a printable pattern.
    Pattern(Pattern),
}

#[derive(Debug, Args)]
struct Validate {
    /// The model to inspect.
    model: PathBuf,
    /// Where to write the health report as JSON.
    #[arg(long)]
    json: PathBuf,
}

#[derive(Debug, Args)]
struct Repair {
    /// The model to repair.
    model: PathBuf,
    #[command(flatten)]
    off: repair::Off,
    /// A boundary loop to fill, by its lowest source vertex `<primitive>.<vertex>`; repeatable.
    /// A loop not listed stays an opening.
    #[arg(long, value_parser = repair::loop_ref)]
    fill: Vec<LoopRef>,
    /// Where to write the repair plan as JSON, so a later run can replay it.
    #[arg(long)]
    plan_out: Option<PathBuf>,
    /// Where to write the repaired model, as OBJ.
    #[arg(short, long)]
    out: PathBuf,
}

#[derive(Debug, Args)]
struct Pattern {
    /// The model to flatten.
    model: PathBuf,
    /// The seam set, as JSON.
    #[arg(long)]
    seams: PathBuf,
    /// The fabric preset by name.
    #[arg(long)]
    fabric: String,
    /// The target finished size, in millimetres.
    #[arg(long, value_parser = positive_mm)]
    size_mm: f64,
    /// The sheet size the print export tiles onto.
    #[arg(long, value_enum)]
    page: Page,
    /// The output directory.
    #[arg(short, long)]
    out: PathBuf,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Page {
    A4,
    Letter,
}

/// A finite length in millimetres greater than zero.
fn positive_mm(text: &str) -> Result<f64, String> {
    match text.parse::<f64>() {
        Ok(mm) if mm.is_finite() && mm > 0.0 => Ok(mm),
        _ => Err(format!("{text:?} is not a finite length above 0 mm")),
    }
}

fn main() -> ExitCode {
    let outcome = match Cli::parse().command {
        Command::Validate(v) => validate::run(&v.model, &v.json),
        Command::Repair(r) => repair::run(
            &r.model,
            &repair::plan(&r.off, &r.fill),
            &r.out,
            r.plan_out.as_deref(),
        ),
        Command::Pattern(_) => {
            eprintln!("openplush-cli: the pattern stage is not built yet");
            return ExitCode::FAILURE;
        }
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            report(&failure);
            ExitCode::FAILURE
        }
    }
}

fn report(failure: &Failure) {
    eprintln!("openplush-cli: {failure}");
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn the_command_definition_is_consistent() {
        Cli::command().debug_assert();
    }
}
