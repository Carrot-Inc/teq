# The reduction survey

Where the tree can be made smaller by sharing an implementation, by a structure that states once what code
repeats, or by a well-known dependency in the place of teq's own: measured against one baseline, each candidate
held to the contract it would have to keep. The survey decides nothing; `candidates.md` ranks what it found and
the owner decides each candidate. What is here:

- `survey.scala`: the baseline's measurements (its header has the parts and the flags), written under
  `out/survey/`: tokei's counts, jscpd's clones at 50 and 100 tokens with their families, the functions of
  `src/` with their spans, the registry's metadata of the crates `crates.txt` names. `jscpd/` pins jscpd.
- `oracle.scala`: the JDK's answers the contract gates below hold teq's implementations to.
- `candidates.md`: the candidates, ranked, each with its evidence and its status.
- `evidence/`: what the statuses stand on, as text: the baseline's totals, the probes and their outputs, each
  spike's coverage manifest with the summaries of its runs, the commands, the binaries' digests (its README).

## Running

```
./teq interp bench/survey/survey.scala -- [--only tokei|clones|functions|crates]...
scala-cli run -S 3.8.4 --jvm system --server=false bench/survey/oracle.scala
./teq interp bench/survey/survey.scala -- --check      # the script's own checks: the function scan, the baseline's reuse
```

The survey measures the working directory's tree. A revision's figures come from a checkout of it with the script
named from another, so that the survey's own files are not counted:
`git worktree add --detach <dir> <sha>`, then from `<dir>`,
`./teq interp <this>/bench/survey/survey.scala -- --out <this>/out/survey --crates <this>/bench/survey/crates.txt --jscpd <this>/bench/survey/jscpd`.
tokei 15.0.0 (`cargo install tokei --version 15.0.0 --locked`), node and npm (jscpd 5.4.1 from the lock, installed
under the output directory) and curl are what it needs.

## The baseline

At master `0e2ef0f12`, tokei 15.0.0, jscpd 5.4.1, the measurements of 2026-10-17 (`evidence/baseline/baseline.json`):

| Tree | Language | Files | Code lines | Comment lines | Physical lines (tokei) | Raw lines (`wc -l`) |
|---|---|---:|---:|---:|---:|---:|
| `src/` | Rust | 228 | 264,533 | 39,495 | 318,633 | 318,751 |
| `std/` | Scala | 117 | 28,349 | 1,933 | 32,589 | 32,589 |
| `tests/` | Scala (fixtures mostly) | 6,603 | 129,785 | 17,301 | 161,684 | 161,675 |
| `tests/` | shell | 96 | 20,152 | 4,089 | 24,760 | 24,760 |
| `tests/` | Python | 36 | 6,335 | 299 | 7,417 | 7,417 |
| `bench/` | Python | 35 | 11,044 | 144 | 12,143 | 12,143 |
| `bench/` | Scala | 116 | 8,004 | 816 | 9,614 | 9,614 |
| `bench/` | shell | 56 | 5,608 | 1,761 | 7,612 | 7,612 |

A Rust file's documentation comments are tokei's embedded Markdown, counted here among its comments with what that
Markdown embeds in its turn (TOML in `src/dialect.rs`, Scala in `src/parser/mod.rs`); tokei's physical lines are its
code, comments and blanks, which need not equal the files' newlines (the raw column). jscpd over
`src/` (227 sources, 318,732 lines by its own count) finds 714 pairs and 5,958 duplicated lines (1.87%) at 50
tokens, 118 pairs and 1,425 lines (0.45%) at 100: detector statistics, not deletions. The scan finds 12,678
functions with bodies (12,640 before it read a signature past an array type's `;`, `[u8; 64]`, which hid 38); the
thirty longest hold 14,385 lines, the same thirty.

## The contract gates

A candidate that replaces an implementation keeps its contract; each gate is a unit test over teq's own code with
the input that tells the contract from what a substitute gives, and a substitute passes it before a spike is paid
for. The oracle is the JDK's where the contract is Java's (`oracle.scala`), a standard's test vectors where it is
a standard's, and teq's own value, a pin, where the contract is teq's output.

| Test | Contract | Discriminating input | Answer | What it turns away |
|---|---|---|---|---|
| `deflate::tests::gives_the_jdks_bytes` | `java.util.zip.Deflater`'s bytes (zlib's) | four inputs at levels 0, 1, 6, 9, raw and wrapped | the JDK's length and CRC-32 of each; `"abcde" * 1000` at 6 byte for byte | miniz_oxide 0.9.1 (12 of the 32 agree), zlib-rs 0.6.8 (18) |
| `zip::tests::crc32_is_the_jdks` | CRC-32 (IEEE) | `123456789`; every split of 100 bytes | `cbf43926` | |
| `jvm::classfile::tests::the_jars_crc32_is_zips` | the jar writer's CRC is the reader's | lengths 0 to 299 | `zip::crc32`'s | |
| `lsp::json::tests::keeps_repeated_keys_in_order` | an object as written; the first of a repeated key | `{"z":1,"a":2,"z":3}` | the same text; `z` is 1 | serde_json (`{"a":2,"z":3}`, 3; with `preserve_order` `{"z":3,"a":2}`) |
| `interp::regex::tests::answers_as_java_util_regex` | `java.util.regex` by UTF-16 index | `(a)\1` on `aa`; `b` in `😀b`; `(?i)é` and `(?iu)é` on `É`; `.` in `😀` from 0 and from 1 | matches; 2; no, yes; 0–2, 1–2 | the `regex` crate (refuses the back reference, says 4) |
| `intern::tests::fx_hasher_reads_a_tail_padded_with_zeros` | the order of every `FxMap` (a pin) | `abc` | `8c41a58af3920c75` | rustc-hash 1.1.0 (`c360d75917ea8923`), any other hasher |
| `write::tests::hash_is_pinned` | a module's footer, `hot-build.mjs`, the products' identity (a pin) | ``, `abc`, `0123456789abcdefg` | `0`, `a649577314915a86`, `c0a0ad83fed0e53e` | |
| `products::tests::fingerprint_is_fnv_1a`, `source::tests::tags_are_fnv_1a`, `jvm::api::tests::fingerprints_are_fnv_1a` | FNV-1a, 32 and 64 bits, persisted | ``, `a`, `foobar` | the FNV test vectors | |
| `task::fetch::tests::sha1_of_the_standard_vectors`, `task::sha256::tests` | SHA-1, SHA-256 (already gated) | the FIPS vectors | | |

## A spike's proof

A spike is proven only by a coverage manifest whose required rows ran; a row that did not run is the spike's
blocker, never a pass, and a prerequisite missing (tests/all.sh counts a skip as passed) is listed as a skip.
The manifest, one per spike, under `evidence/spikes/<name>/manifest.md`:

| Row | Contents |
|---|---|
| Identity | the baseline's and the spike's SHAs, the branch, each binary's profile, `--version` and SHA-256 |
| Net diff | added and removed lines of code, comments, tests, generated code and adapters apart (`git diff --numstat` read by category), overlap with other candidates counted once |
| Unit tests | the touched modules' tests and the gates above: required, run, skipped |
| Warnings | `tests/rust-warnings.sh` (release and checks profiles) |
| Malformed input | under the abort build, where the change reads input |
| Instructions | `bench/instr-compare.sh <master> <spike> 1 depth_1 gview_1 core-only realistic-api realistic-frontend`, the raw counts, each program against 0.5%; an operation-specific measurement where `compiler check` does not run the changed path |
| Identity | `tests/support/identity.sh --produce` and `--compare` (JavaScript, class files, diagnostics) at several worker counts, and the TASTy, analysis-bytes, module, watch and chain identity the change touches |
| Suites | each suite the change touches, with its case counts against master's binary on the same machine |
| Build and size | cold and warm release builds, the ship build's time and peak memory, raw, stripped and packaged sizes, matched before and after, profile and target named |
| Commands | every command as run, with its log under the manifest's directory |
