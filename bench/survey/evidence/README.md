# The reduction survey's evidence

What the survey's report and `../candidates.md` stand on, as text: the baseline at master `0e2ef0f12`, the probes
that screened the dependencies, and each spike's coverage manifest with the summaries of its runs. No binary is
kept; the binaries are named by their digests (`binaries.sha256`). Paths are this tree's, or a role in angle
brackets: `<master>` master's release binary (a `git archive` export of `0e2ef0f12` built with
`GIT_DIR=/nonexistent TEQ_REVISION=0e2ef0f12`), `<spike 1>` and `<spike 3>` the spikes' release binaries, a
`<... worktree>` a checkout of the revision it names. `commands.md` holds every command as it ran. Rustc 1.98.1,
JDK 24.0.2, node 25.9.0, tokei 15.0.0, jscpd 5.4.1, clippy 0.1.98, valgrind's cachegrind for the instruction counts.

| Path | What it holds |
|---|---|
| `baseline/baseline.json` | the survey's totals at master (the function scan as `../survey.scala --check` pins it), the revision and the tools (`../survey.scala`): tokei per tree and language (code, comments, tokei's physical lines with the embedded languages at every depth, the files' raw lines), jscpd at 50 and 100 tokens, the function scan, the registry's crates |
| `baseline/top-families-{50,100}.md`, `longest-30.tsv` | jscpd's ten largest families at each threshold; the thirty longest functions |
| `baseline/crates-summary.json` | the registry's metadata of `../crates.txt`'s crates: version, licence, archive size, release, declared dependencies |
| `baseline/oracle.log` | `../oracle.scala`'s answers under scalac 3.8.4 on JDK 24.0.2: the contract gates' values |
| `baseline/texpr-walkers.txt`, `clippy-counts.txt` | the scan of the typed IR's walkers (candidate 7); clippy's warnings by message (candidate 23) |
| `probes/deflate/` | flate2 1.1.10's three backends against the oracle's 32 deflate outputs (`probe.log`: miniz_oxide 12, zlib-rs 18, C zlib 32), C zlib by libz-sys 1.1.30 with `static` (its feature graph, the binary's dynamic libraries, `zlibVersion()` 1.3.2, the same on a host with system zlib simulated, and the control that links the system's), its lock the resolved graph (candidates 8, 13) |
| `probes/json/` | serde_json 1.0.154 against `lsp/json.rs`'s contract (candidate 11) |
| `probes/refusal/` | a macro reading `Apply.fun` off a literal, under master's binary and spike 3's (spike 3's malformed-input row) |
| `spikes/shared-names/` | spike 1's manifest and its runs: the net diff, unit tests, warnings, the release and ship pairs with their counts, the whole-jar check, identity, products, the scala3 harnesses, interp, stdlib, fold, size, the edit-order suites |
| `spikes/tree-inventory/` | spike 3's: the same rows, the macro-heavy pair, the names and messages kept, classpath |
| `spikes/case-runner/` | spike 2's: the planted and whole-corpus runs of the old and new runners, cancellation, the harness apart from its binary, the gate's own checks; `recipe/` replays the planted phases on both runners (`plant.scala`, `subset.txt`, `planted/`), `recipe-run/` its run |
| `spikes/identity-jars-note.txt` | the two programs identity.sh finds different on every pair, their timing reports alone |
| `feat/` | this branch's release binary against master's: the pair, the typing harness, the byte comparison |
| `ship/sizes.txt` | the ship profile's builds of master and spikes 1 and 3: wall time, peak RSS, raw, stripped and gzip sizes |
