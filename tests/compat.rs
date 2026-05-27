//! Exit-code parity vs `samtools quickcheck` 1.23.1 on a valid BAM and three
//! corrupted variants. Skipped if samtools is not on PATH.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const GOLDEN_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden");

fn samtools_available() -> bool {
    Command::new("samtools")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_rsomics-bam-quickcheck")
}

fn run(cmd: &str, args: &[&str], path: &Path) -> bool {
    Command::new(cmd)
        .args(args)
        .arg(path)
        .status()
        .expect("spawn")
        .success()
}

fn ensure_valid_bam(dir: &Path) -> PathBuf {
    let dst = dir.join("ok.bam");
    if dst.exists() {
        return dst;
    }
    // Build a tiny BAM via samtools from an inline SAM (only used if samtools is
    // available; the test self-skips otherwise).
    let sam = dir.join("ok.sam");
    std::fs::write(
        &sam,
        "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\nr1\t0\tchr1\t1\t60\t10M\t*\t0\t0\tACGTACGTAC\tIIIIIIIIII\n",
    )
    .unwrap();
    let ok = Command::new("samtools")
        .args(["view", "-b", "-o"])
        .arg(&dst)
        .arg(&sam)
        .status()
        .expect("samtools view")
        .success();
    assert!(ok, "samtools view failed to build golden BAM");
    dst
}

fn corrupt_copy(src: &Path, dst: &Path, action: impl FnOnce(&mut File)) -> PathBuf {
    std::fs::copy(src, dst).unwrap();
    let mut f = OpenOptions::new().read(true).write(true).open(dst).unwrap();
    action(&mut f);
    dst.to_path_buf()
}

#[test]
fn exit_code_parity_vs_samtools_quickcheck() {
    if !samtools_available() {
        eprintln!("SKIP: samtools not found");
        return;
    }
    std::fs::create_dir_all(GOLDEN_DIR).unwrap();
    let dir = Path::new(GOLDEN_DIR);
    let ok = ensure_valid_bam(dir);

    // Case 1: valid BAM → both succeed.
    assert!(run(bin(), &[], &ok), "ours should accept valid BAM");
    assert!(
        run("samtools", &["quickcheck"], &ok),
        "samtools should accept valid BAM"
    );

    // Case 2: drop the BGZF EOF (truncate last 28 bytes) → both fail.
    let truncated = corrupt_copy(&ok, &dir.join("no_eof.bam"), |f| {
        let len = f.metadata().unwrap().len();
        f.set_len(len - 28).unwrap();
    });
    assert!(!run(bin(), &[], &truncated), "ours should reject no-EOF");
    assert!(
        !run("samtools", &["quickcheck"], &truncated),
        "samtools should reject no-EOF"
    );

    // Case 3: corrupt BGZF magic at byte 0 → both fail.
    let bad_magic = corrupt_copy(&ok, &dir.join("bad_magic.bam"), |f| {
        f.seek(SeekFrom::Start(0)).unwrap();
        f.write_all(b"XXXX").unwrap();
    });
    assert!(!run(bin(), &[], &bad_magic), "ours should reject bad magic");
    assert!(
        !run("samtools", &["quickcheck"], &bad_magic),
        "samtools should reject bad magic"
    );

    // Case 4: with --no-eof, the truncated file should pass ours (samtools `-u`
    // means something different — "unmapped" — so we don't cross-check this
    // flag; we only assert ours' own semantics).
    assert!(
        run(bin(), &["--no-eof"], &truncated),
        "ours --no-eof should accept truncated BAM with intact header"
    );

    // Sanity: a totally empty file fails both.
    let empty = dir.join("empty.bam");
    std::fs::write(&empty, b"").unwrap();
    assert!(!run(bin(), &[], &empty), "ours should reject empty");
    assert!(
        !run("samtools", &["quickcheck"], &empty),
        "samtools should reject empty"
    );

    // Read a known-valid file's first 4 bytes (sanity, not a test): the BGZF
    // magic should be present.
    let mut head = [0u8; 4];
    File::open(&ok).unwrap().read_exact(&mut head).unwrap();
    assert_eq!(head, [0x1f, 0x8b, 0x08, 0x04]);
}
