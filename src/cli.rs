use std::path::PathBuf;

use clap::Parser;
use rsomics_common::{CommonFlags, Result, Tool, ToolMeta};

use rsomics_bam_quickcheck::{QuickcheckOpts, quickcheck};

pub const META: ToolMeta = ToolMeta {
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
};

#[derive(Parser, Debug)]
#[command(
    name = "rsomics-bam-quickcheck",
    version,
    about = "Quickly validate BAM file integrity — port of samtools quickcheck"
)]
pub struct Cli {
    /// Input BAM file(s). Exit code 0 = all valid; non-zero = at least one invalid.
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,

    /// Print one diagnostic per failing file (samtools `-v`; CommonFlags owns `-v/--verbose`,
    /// so we use a long-only flag here).
    #[arg(long = "print-errors")]
    pub print_errors: bool,

    /// Skip the BGZF EOF check (the `-u` / `--unmapped` flag in samtools).
    #[arg(short = 'u', long = "no-eof")]
    pub no_eof: bool,

    #[command(flatten)]
    pub common: CommonFlags,
}

impl Tool for Cli {
    fn meta() -> ToolMeta {
        META
    }

    fn common(&self) -> &CommonFlags {
        &self.common
    }

    fn execute(self) -> Result<()> {
        let opts = QuickcheckOpts {
            no_eof: self.no_eof,
        };
        let mut failed = 0u32;
        for p in &self.inputs {
            if let Err(e) = quickcheck(p, &opts) {
                failed += 1;
                if self.print_errors {
                    eprintln!("{}: {e}", p.display());
                }
            }
        }
        if failed > 0 {
            std::process::exit(1);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_debug_assert() {
        Cli::command().debug_assert();
    }
}
