//! `openplush-cli`: every engine capability as file-in, file-out.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};

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
    /// Repair a model, writing the plan so the repair is reproducible.
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
    /// Merge coincident vertices.
    #[arg(long)]
    weld: bool,
    /// Make face winding consistent.
    #[arg(long)]
    fix_orientation: bool,
    /// How to treat boundary loops.
    #[arg(long, value_enum)]
    fill_holes: Option<FillHoles>,
    /// Where to write the repair plan as JSON.
    #[arg(long)]
    plan_out: PathBuf,
    /// The repaired model.
    #[arg(short, long)]
    out: PathBuf,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FillHoles {
    /// Let the engine classify each loop as an opening or an accidental hole.
    Auto,
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
    /// The finished length of the model's longest dimension, in millimetres.
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

impl Command {
    /// The engine stage this command needs.
    fn stage(&self) -> &'static str {
        match self {
            Self::Validate(_) => "validate",
            Self::Repair(_) => "repair",
            Self::Pattern(_) => "pattern",
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    eprintln!(
        "openplush-cli: the {} stage is not built yet",
        cli.command.stage()
    );
    ExitCode::FAILURE
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
