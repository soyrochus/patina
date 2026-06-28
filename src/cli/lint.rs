use anyhow::Result;
use clap::Args;

use crate::config::PatinaConfig;
use crate::lint;
use crate::output::{JsonEnvelope, print_json};

#[derive(Debug, Args)]
pub struct LintArgs {
    #[arg(long)]
    pub json: bool,
}

pub fn run(args: LintArgs, config: &PatinaConfig) -> Result<()> {
    let report = lint::run_lint(config)?;
    let ok = report.errors.is_empty();

    if args.json {
        let envelope = JsonEnvelope::new(
            "lint",
            ok,
            Some(report.data()),
            report.warnings,
            report.errors,
        );
        print_json(&envelope)?;
    } else {
        for warning in &report.warnings {
            println!("warning: {}", warning.message);
        }
        for error in &report.errors {
            println!("error: {}", error.message);
        }
        if ok {
            println!(
                "lint clean: {} Markdown file(s) checked",
                report.files_checked
            );
        } else {
            println!(
                "lint found {} error(s) across {} Markdown file(s)",
                report.errors.len(),
                report.files_checked
            );
        }
    }

    Ok(())
}
