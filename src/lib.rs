//! Quickly validate a BAM file — port of `samtools quickcheck`.
//!
//! Checks performed (matches samtools 1.23.1 behavior):
//! 1. The file begins with a valid BGZF block magic (`1f 8b 08 04`).
//! 2. The first BGZF block decompresses and starts with the BAM header magic
//!    `BAM\1` (`42 41 4d 01`).
//! 3. The file ends with the 28-byte BGZF empty-block EOF marker
//!    (`1f 8b 08 04 00 00 00 00 00 ff 06 00 42 43 02 00 1b 00 03 00 00 00 00 00 00 00 00 00`)
//!    — unless `--no-eof` is set.
//!
//! Exit code 0 = all input files valid; non-zero = at least one invalid.
//!
//! ## Origin
//! Independent Rust reimplementation of `samtools quickcheck`. samtools is
//! MIT-licensed; the precise BGZF EOF marker bytes and the "BAM\1" header magic
//! check were determined from samtools' MIT-licensed source plus the SAM/BAM
//! and BGZF specifications.
//!
//! License: MIT OR Apache-2.0. Upstream credit: samtools (MIT).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use flate2::read::MultiGzDecoder;

/// The 28-byte BGZF EOF (empty-block) marker per the SAM/BAM specification.
pub const BGZF_EOF: [u8; 28] = [
    0x1f, 0x8b, 0x08, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x06, 0x00, 0x42, 0x43, 0x02, 0x00,
    0x1b, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[derive(Debug, Default, Clone, Copy)]
pub struct QuickcheckOpts {
    /// Skip the BGZF EOF check (the `-u` / `--no-eof` flag).
    pub no_eof: bool,
}

#[derive(Debug)]
pub enum QuickcheckError {
    Io(std::io::Error),
    BadBgzfMagic,
    BadBamMagic,
    MissingEof,
    Truncated,
}

impl std::fmt::Display for QuickcheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::BadBgzfMagic => f.write_str("bad BGZF magic (not a BGZF/BAM file)"),
            Self::BadBamMagic => f.write_str("bad BAM header magic"),
            Self::MissingEof => f.write_str("missing BGZF EOF marker"),
            Self::Truncated => f.write_str("file too short to be a valid BAM"),
        }
    }
}

impl std::error::Error for QuickcheckError {}

impl From<std::io::Error> for QuickcheckError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Run the quickcheck on a single BAM. Returns `Ok(())` on success.
pub fn quickcheck(path: &Path, opts: &QuickcheckOpts) -> Result<(), QuickcheckError> {
    let mut f = File::open(path)?;
    let meta = f.metadata()?;
    let len = meta.len();

    if len < 28 {
        return Err(QuickcheckError::Truncated);
    }

    // 1) BGZF magic at offset 0.
    let mut head = [0u8; 4];
    f.read_exact(&mut head)?;
    if head != [0x1f, 0x8b, 0x08, 0x04] {
        return Err(QuickcheckError::BadBgzfMagic);
    }
    // Rewind for BAM magic check.
    f.seek(SeekFrom::Start(0))?;

    // 2) Decompress just enough to read the BAM header magic ("BAM\1").
    // BGZF is multi-member gzip; MultiGzDecoder concatenates blocks transparently.
    let mut bam_magic = [0u8; 4];
    MultiGzDecoder::new(&mut f).read_exact(&mut bam_magic)?;
    if bam_magic != *b"BAM\x01" {
        return Err(QuickcheckError::BadBamMagic);
    }

    // 3) BGZF EOF marker — last 28 bytes.
    if !opts.no_eof {
        let mut tail = [0u8; 28];
        f.seek(SeekFrom::End(-28))?;
        f.read_exact(&mut tail)?;
        if tail != BGZF_EOF {
            return Err(QuickcheckError::MissingEof);
        }
    }

    Ok(())
}
