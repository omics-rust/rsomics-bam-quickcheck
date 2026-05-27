# rsomics-bam-quickcheck

Quickly validate that one or more BAM files appear intact — a Rust port of
`samtools quickcheck`. Checks: (1) BGZF magic at the start, (2) `BAM\1` header
magic after decompressing the first BGZF block, (3) the 28-byte BGZF EOF marker
at the end. Exits non-zero if any file fails.

## Install

```sh
cargo install rsomics-bam-quickcheck
```

## Usage

```sh
rsomics-bam-quickcheck file.bam                # exit 0 = OK
rsomics-bam-quickcheck *.bam                   # check many; exit non-zero on any failure
rsomics-bam-quickcheck -v file.bam             # also print one-line diagnostic per failure
rsomics-bam-quickcheck -u file.bam             # skip the EOF check
```

| flag | meaning |
|---|---|
| `-v, --verbose` | print a diagnostic line per failing file (default: silent) |
| `-u, --no-eof`  | skip the BGZF EOF-marker check |

## Origin

Independent Rust reimplementation of `samtools quickcheck`. samtools is
MIT-licensed; the precise BGZF EOF marker bytes, the `BAM\1` header magic, and
the silent-by-default exit-code convention were determined from samtools'
MIT-licensed source plus the SAM/BAM and BGZF specifications.

License: MIT OR Apache-2.0.
Upstream credit: [samtools](https://github.com/samtools/samtools) (MIT).
