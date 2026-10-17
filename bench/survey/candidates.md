# The candidates

Ranked by eligibility first (the contract kept, dotty's mechanism kept), then the benefit to a reader, the
authored code removed, the strength of the evidence, the effort to make and to prove it, and the risk. Every
figure is labelled by its kind: **measured** (a diff that was made, or a tool's count), **clone** (jscpd's tokens
and span, which no diff has confirmed) or **estimate** (a reading, named as such, never multiplied by a
confidence). The net column is in physical lines, the code lines (neither blank nor a comment) beside them where a
diff or a span was counted, tests and generated code apart; overlap is counted once, in the earlier row. A status is one of
*proven* (a spike's coverage manifest ran whole), *plausible* (eligible, not yet proven), *pending qualification*
(its gate passed, a dependency's qualification not done), *rejected* (with the input that rejects it), *deferred*
(a part of its own if the owner wants it) or *not a candidate*. The survey of 2026-10-17 at master `0e2ef0f12`;
`evidence/` holds what each status stands on (the spikes' manifests under `evidence/spikes/`).

| # | Candidate | Sites | Net physical lines (code) | Evidence | Gate | Status |
|---:|---|---|---:|---|---|---|
| 1 | The quoted trees' fields, one `fields!` inventory per kind | `interp/quoted.rs` `install_trees`: 46 registrations of 15 kinds | −258 (code −263) | measured (`spike/reduction-tree-inventory`, `evidence/spikes/tree-inventory/`) | every name and message of the kinds; identity, the macro suites, interp, classpath | proven |
| 2 | The primitive operators' Scala names in `tir.rs`; the jar's CRC-32 by zip.rs's | `interp/quoted.rs` 2829, `tasty/write/pickle/term.rs` 6803, `typer/deps.rs` 632; `jvm/classfile.rs` 1143–1161 | −79 (code −76) | measured (`spike/reduction-shared-names`, `evidence/spikes/shared-names/`) | TASTy, analysis, class files and whole jars byte for byte; `crc32_is_the_jdks` | proven |
| 3 | FNV-1a once | `products.rs` 51, `source.rs` 114, `jvm/api.rs` 425 and 3821 | about −10 | estimate (under jscpd's 50 tokens) | the three `fnv_1a` tests; products and analysis identity | plausible |
| 4 | One stored-zip writer | `jvm/classfile.rs` 1072–1141, `task/stage.rs` 195–240 | about −55 | estimate | both writers' whole-file bytes | plausible |
| 5 | A child's stderr tail by one reader | `task/resident.rs` 71–85, `task/runner.rs` 295–309 | about −12 | clone (193 tokens) | `tests/task.sh` | plausible |
| 6 | The typer's repeated setups, seven families | `typer/members.rs` 2348/2382 and the others of jscpd's top ten | about −80 | clone (about 150 lines) | dotty's mechanism per family; the typing harness, identity | plausible |
| 7 | One child visitor of `TExpr` | 13 walkers | under −100 | measured lower bound: 143 recursion-only arm lines; the visitor 60–80 | each walker's order of visits | plausible, low value for the risk |
| 8 | C zlib (libz-sys 1.1.30, bundled 1.3.2, static) for deflate and inflate | `deflate.rs` 1–1008, zip.rs 210–740 (the inflaters, Adler-32) and 159–208 (CRC-32): 1,589 physical, 1,467 code | about −1,490 (code about −1,370) | measured spans and gate (32 of 32 of the JDK's outputs with libz-sys's `static` enabled: no libz linked, `zlibVersion()` 1.3.2, `evidence/probes/deflate/`), estimated adapter (about 100); the tests (deflate's 18 lines and stage 1's gate, zip.rs's 72) kept as the adapter's gates | `gives_the_jdks_bytes`, the inflater's pieces and zlib's messages | pending qualification |
| 9 | The suites over `tests/cases` on a shared Scala case runner | `tests/run_stdlib.sh` converted; `run_interp.sh`, `run_jvm.sh` next | +79 for one suite (code +52) | measured (`spike/reduction-case-runner`, `evidence/spikes/case-runner/`; its planted phases replayable by `evidence/spikes/case-runner/recipe/plant.scala`, run in `recipe-run/`) | verdicts, lines, files, exits, `--update`, skips, cancellation | proven for one suite; a robustness lane, not a reduction |
| 10 | `sha1` 0.11.0, `sha2` 0.11.1 | `task/fetch.rs` 626–691 (66 physical, 61 code), `task/sha256.rs` 1–93 (93, 84) | about −145 (code about −130: 145 before a wrapper of about 15 keeping `new`, `update`, `hex`) | measured spans, estimated adapter; the tests (SHA-256's 16 lines, SHA-1's vector test) kept as the gates | the FIPS vectors | plausible, poor economics |
| 11 | `serde_json` 1.0.154 | `lsp/json.rs` (425) | | measured: repeated keys merged, nesting refused from 128 (teq 512), a lone surrogate refused | `keeps_repeated_keys_in_order` | rejected as a drop-in |
| 12 | `crc32fast` 1.5.2 | zip.rs's CRC | | 2 needs no dependency | | rejected |
| 13 | miniz_oxide 0.9.1, zlib-rs 0.6.8 for deflate | `deflate.rs` | | measured: 12 and 18 of the JDK's 32 outputs | `gives_the_jdks_bytes` | rejected |
| 14 | miniz_oxide for inflation | zip.rs's inflaters | | its status codes are not zlib's messages | | rejected; 8 keeps the messages |
| 15 | `regex` 1.13.1, `fancy-regex` 0.19.2 | `interp/regex.rs` (1,288) | | measured: the back reference refused, a byte index for a UTF-16 one | `answers_as_java_util_regex` | rejected |
| 16 | `yaml-rust2` 0.13.0 | `task/lock.rs` (864), copied into Zed | | the grammar's refusals, the writer and the copy stay | | rejected for the survey |
| 17 | `ureq` 3.4.2 | curl in `task/fetch.rs` | | new TLS, roots, proxies; native code | | rejected for the survey |
| 18 | `rustc-hash`, `ahash`, `fnv` | `intern.rs`'s FxHasher, the FNV folds | | the hasher is a pin | `fx_hasher_reads_a_tail_padded_with_zeros` | rejected |
| 19 | `mimalloc` 0.1.52 | `alloc.rs` | | the resident's lifecycle stays | | deferred |
| 20 | `bumpalo`, `typed-arena` | `arena.rs` | | no drop-in | | deferred |
| 21 | Unicode property crates | `interp/char_tables.rs` (generated) | | | | not a candidate |
| 22 | The thirty longest functions (14,385 lines; of 12,678) | `evidence/baseline/longest-30.tsv` | | measured spans | | deferred, readability lane |
| 23 | clippy's 1,157 warnings (`-W clippy::all`) | 685 `map_or(false, …)`, 913 auto-fixable | about 0 | measured | | readability lane |

Not utilities to replace, and not counted: `interp/net.rs`, `memory.rs`, `measure.rs`, `crew.rs`, `interp/natives.rs`.
