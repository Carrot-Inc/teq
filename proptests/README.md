# proptests

Property-based tests of teq on [Hegel](https://hegel.dev) (`hegeltest`, Hypothesis's engine
for Rust). A crate of its own: the compiler keeps no dependencies, so this crate has its own
manifest, lock file (`hegeltest` pinned, `static-engine` so that the one lock governs all of
its 42 packages) and target directory, and drives the `teq` binary named by `TEQ` (default
`../target/release/teq`) as the shell suites do. It reads nothing of `src/`.

```
cargo fetch --locked                      # once, from crates.io; everything after is offline
TEQ=../target/release/teq cargo test --offline --locked -- --test-threads=1
../tests/prop.sh                          # the same with 100 cases and seed 1 per property
```

The properties, each stated in its source's header:

- `tests/session.rs`: a resident session (`teq compiler watch --split`, `teq compiler watch --check`) answers
  and writes what a fresh build of the same sources does, under generated edit sequences
  (`src/edits.rs` the rules, `src/model.rs` the programs, `src/oracle.rs` the invariant per
  outcome, `src/driver.rs` the session and its comparison).
- `tests/targets.rs`: generated expressions print the same on JavaScript, in the interpreter
  and on the JVM, folded and at run time, and under scalac 3.8.4 for a sample and every
  disagreement (`src/exprs.rs`, `src/targets.rs`); one in eight is a regular expression's
  operation over a generated pattern and input (`src/regexes.rs`). An input may be several programs, which
  scalac is asked about in one run (`TEQ_PROP_PROGRAMS`); `scalac_batched_answers_as_alone`,
  ignored by default, checks that the batch answers as the programs alone do and measures both.
- `tests/order.rs`: the order of the input files and directories does not change what
  `teq compiler check` and `teq compiler build --split` answer (`src/order.rs`).
- `tests/controls.rs`: the session property over `tests/split/retype` for the controls on
  known defects, run by hand against a chosen binary (`src/fixture.rs`).
- `tests/footprint.rs`: two trivial properties, for measuring the crate's cost.

`src/known.rs` lists the defects found that master still has, each with the precondition that
keeps its trigger out; `TEQ_PROP_KNOWN=<name>,<name>` puts triggers back. What each exclusion
takes out of the generators beside its trigger, as of the first campaign (2026-09-29):

| Exclusion | Scope against the defect |
|---|---|
| `macro-val-order` | Exact: the val's file is named to sort first, and tests/split/retype stays out of the order property's corpora, since its macros read two of its files. |
| `macro-reads-edited-file` | Exact: every site of the three files the fixture's macros may read is a trigger (`pick`'s argument can turn to any word, after which `right.scala` is read too). |
| `file-change-with-edit` | Exact: a file's addition or removal is two builds, the definitions first and the file after, each well typed. |
| `inline-signature-uncalled` | Exact: the model withdraws an inline method's broken signature while nothing calls it, and shows it otherwise. Asking for the defect does not show it: its trigger is an error that stands while the method's last call is taken out, which a check session's signature errors, going in alone, never do, and split sessions answer as a fresh build does; `src/known.rs` has the case by hand. |
| `signature-error-lost` | Wider in one way, and that way not narrowable in the generator: in a check session an error in a signature or an import goes in alone, repaired in the next build with nothing else changed or standing as the history's last step, so its introduction and its recovery are built. An error that stands while other edits are made is kept out, since the trigger is any retype of the file, which the model cannot foresee: on 68d62cff a session after a failed build retypes files that call nothing that changed, and a generator that froze such a file and everything it calls met the defect within 27 histories. Split sessions have both kinds of error interleaved with every edit. |
| `macro-under-receiver` | Wider: an inline method of an object or a class has a body of literals, parameters and locals, since any call from it may reach a macro through what the expansion types on demand. |
| `inline-val-members` | Exact: the literal type is written only in programs that declare an `inline val`, and every use of one as an operand is a trigger. |
| `inline-val-float` | Exact: every `inline val` of a `Float` is a trigger; `Float` operands are literals and `final val`s. |
| JavaScript's number text | Exact: the `.t` lines are compared without JavaScript; its bits and its two forms are compared. |
| `regex-comments` | Exact for what the generator writes (white space or a comment inside a class, a comment ended by `\r`, NEL, LS or PS, white space before a quantifier and inside its braces); the other places the JDK reads past white space (after a group's `(`, inside an escape or a property's name) are not generated and stand in `tests/cases/regex_comments.scala`. |
| `regex-properties` | Wider: where a pattern names any property its input loses the five characters on which Rust's properties and the JDK's categories part, and `Lu` and `Ll` are left out under `(?i)` whatever the input. |

A guard of the generated expressions stands in the program's text around the operation it
guards, the operation in place: in the folded form the operation's operands are still
constants for the typer to fold.

Settings: `HEGEL_TEST_CASES` and `HEGEL_SEED` (an integer; `none` asks the engine for a fresh
seed, and an empty variable is the same as none set, so a seed a profile or a CI environment
gives stands), `HEGEL_DATABASE` (`.hegel/examples` by default; every campaign run should have
one of its own, since the failures kept there replay before generation whatever the seed),
`TEQ_PROP_WORK` (where the properties write, `../out/proptests` by default),
`TEQ_PROP_STEPS` (the steps of a history, 30), `TEQ_PROP_CASES` (the expressions per program,
40), `TEQ_PROP_EXPRS` (`regex`: every expression a regular expression's, `plain`: none), `TEQ_PROP_PROGRAMS` (the programs of an input, 1; scalac is asked about those on which the
targets agree in one `scala-cli` run, each program in a package of its own, and a program that
throws there or a batch that is refused is asked alone, so a refusal is the one program's),
`TEQ_PROP_SCALAC` (`always`, `never`, or one program in `TEQ_PROP_SCALAC_EVERY`),
`TEQ_PROP_SCALAC_CACHE` (where scalac's answers are kept by the program's text, `<work>/scalac`),
`TEQ_PROP_SCALAC_JVM` (the JVM scalac's programs run on, `system`: the one the JVM target runs
on; `scala-cli` would take a JDK 17 of its own where `JAVA_HOME` is unset, as on the remote
machines, and JDK 17 prints some `Float` and `Double` values otherwise than JDK 19 and later),
`TEQ_PROP_NO_JVM`, `TEQ_PROP_NO_NODE`, `TEQ_PROP_NO_CACHE` (run every history, even one that
passed before in the process), `TEQ_PROP_REPLAYS` (how often an input is replayed when its
verdict changes, and the property's sample, 20; 0 for none), `TEQ_PROP_CORPUS` (another root
of programs for the order property, `TEQ_PROP_CORPUS_ONLY` for it alone),
`TEQ_PROP_ANSWER_SECONDS` and `TEQ_PROP_FRESH_SECONDS` (the bounds on teq's processes),
`TEQ_PROP_UNTIL` (seconds since the epoch after which a property returns before drawing, which
ends the run; a campaign's pieces, never a shrink run), `TEQ_PROP_REDUCE` (a failing program of
the generated expressions is reduced to the cases that fail by themselves, `reduced.scala`
beside the kept program: the cases whose lines differ, together, or the first case the
refusing target refuses alone; a replay of the shrunk input asks for it, since the shrinker
leaves most of a program's cases in place), `TEQ_PROP_KEEP` (a failure kind, as a report's
`the failure's kind:` line names it: a failure of another kind counts as a pass, which keeps a
shrink run on the failure it was given, since the engine tells failures apart by the place of
the panic alone).

A failing run prints the shrunk history in words and a `#[hegel::reproduce_failure("...")]`
line, and keeps the history's files under `<work>/<test>/failures/<started>-<pid>-<n>/` (what
every step wrote and removed, the command it sent, `report.txt`); `docs/DEVELOPING.md`,
"Property-based tests", says how a failure is replayed and what is committed of one. A history
is run once its last rule ran, and one that passed before in the process is not run again
while the engine shrinks: the shrinker asks for the same histories many times over. One that
failed before runs with no cache, as the engine's confirmation of the minimal failing input
does. An input is replayed twenty times with the harness held still the first time its verdict
changes in the process (it failed and then passed, passed and then failed, or failed with
another report); besides, the first input of a property and process that fails twice with one
report is replayed as a sample. Each replay is counted as the original failure reproduced, a
failure with another report or a pass; the counts go to the failure's output and
`<work>/replays.txt`, each replay's outcome to `<work>/replays/`. A rate is of the input it
names (`input <key>`, the last line of the saved failure's `report.txt`), one the shrinker
reached: it says nothing of the minimal input the engine reports at the end, which is replayed
only if its own verdict changes (`src/confirm.rs`).
Coverage figures go to `<work>/stats/<test>.jsonl`, one line per history; its
`generator_errors` counts the builds that failed with no fault injected, which are the
generator's and not the compiler's.

A run under `tests/prop.sh` has 340 s (`PROP_BOUND`) for a property, and the engine gives
shrinking 300 s of its own after a failure is found: a property that `timeout` cuts (exit 124,
no failure report, no `reproduce_failure` line) either found a failure late and was shrinking
it, and then its database holds that failure, or ran long without one (an input that hangs, or
a slow machine), and then it holds none; the log says which input ran last. The
database keeps the smallest failure reached, and a rerun replays it and reports it as it is:
the engine (hegeltest-c 0.44.1) takes a kept failure whose choices replay exactly as already
shrunk. To shrink it further, `tests/prop-campaign.sh requeue <database>` moves the kept
failures to the key's secondary entries, which the engine replays as failures to shrink, and
a rerun with a bound that leaves the shrinker its time, `PROP_BOUND=900 ./tests/prop.sh` or
from the crate `TEQ=... cargo test --test <name>`, shrinks from there.

## Campaigns

`tests/prop-campaign.sh` runs the properties for a wall-time budget: short discovery pieces
on fresh seeds with shrinking off, several at a time, each one test of a property with its
own seed, database and work directory, scalac asked about every program in batches of 20 and
its answers shared; a failure is shrunk as soon as its piece ends, before further pieces start,
the first of each coarse signature first (the targets that print alike, a refusal's target and
message, a session's kind of build and what differs) and every other after them while time
remains, since two defects can share a kind, a shrunk
program of the generated expressions replayed once more to reduce it to its failing cases;
pieces stop in time for the last failures to be shrunk; then `report.md`: every shrunk failure
with its test, seed, piece, shrunk report, kept program or history and `reproduce_failure`
attribute, whether it is a known exclusion's (when `PROP_KNOWN` put triggers back: the shrunk
input passes once they are out again), the inputs run per property, scalac's cost per program
batched and alone, and the cron line that runs it nightly. `here <seconds>` runs it where it stands;
`start <seconds> [machine]`, `status <handle>` and `collect <handle>` run it on a remote machine
through the scripts `REMOTE_AGENT` names (claimed, mirrored, teq built there, the directory fetched to
`out/campaign/<stamp>` and the machine released), and `nightly <seconds>` chains the three,
for a cron line. A failure is named by its kind (`TEQ_PROP_KEEP` holds a shrink run to it); a
program a target does not finish in its bound is reported apart and not shrunk, and a piece
cut by its own bound is incomplete, reported apart from the failures with its log and work
directory kept. It exits 0 where nothing failed, 1 where something did, 3 where nothing failed
but a piece or a shrink run was cut; `collect` and `nightly` exit with the campaign's status, 1
where the fetch fails and 3 where the campaign did not end. Its settings are in the script's
header; `tests/prop-scripts.sh` checks what it decides by itself (the shrink queue, `requeue`
beside a save in flight, the classification's environment, the statuses) without a campaign. A failure found is reduced by hand:
a defect of the compiler becomes a task-list item and an exclusion here (`src/known.rs`,
its narrowest precondition), a defect of the harness is repaired.

## Retiring an exclusion

`tests/prop-known.sh <name>` runs the properties whose sources ask for the name with its
trigger put back (`TEQ_PROP_KNOWN=<name>`) against `TEQ`, 200 programs or histories on seeds 1
and 2, scalac asked about every program in batches of 20: a failure means the defect stands, a
run cut by its bound decides nothing. A pass there is coverage, not the evidence: with
`TEQ_BEFORE` the binary before the repair, the trigger must fail there, and its first failure,
shrunk and kept to its kind (`TEQ_PROP_KEEP`), is the witness, which must pass on `TEQ_BEFORE`
once the trigger is out again (a shrinker may trade the trigger's failure for another defect's of
the same kind) and on `TEQ`; a
trigger of the generated expressions then becomes `tests/cases/prop_<name>.scala`, the witness
reduced to its failing cases, with scalac's expected output (a Scala.js case with
`tests/jvm-expected/` beside it when it prints a floating value's text), which must pass on the
three targets and is listed for the interpreter and the JVM. The script exits 0 only then, 1
where the defect stands or a step fails, 3 where the trigger passes but nothing is retired (no
`TEQ_BEFORE`, or a session trigger, whose witness is a history and whose case a split-watch
scenario written by hand). What is left by hand, in one commit with the case: the guard (the
script lists the sites that ask for the name), the entry in `src/known.rs`, its row above and
its task-list item. `inline-signature-uncalled` has no trigger the generators reach, so a
pass says nothing of it, and it stays until its repair lands with a case by hand.

A rare trigger may not fail within the defaults before the repair: the run there has one
program to an input and asks scalac about one program in `TEQ_PROP_SCALAC_EVERY` (10), so a
defect the three targets share, which scalac alone tells, is seen in a tenth of the programs
that reach it. Find a seed first, the targets property with the trigger asked against
`TEQ_BEFORE`, `TEQ_PROP_PROGRAMS=1` and the same settings, and give it with `KNOWN_SEEDS` and
`KNOWN_INPUTS` past its failing input; the settings pass through to both runs (2026-09-29: three
of four triggers needed a seed, and `boolean-and-or-short-circuit` `TEQ_PROP_CASES=200` and
`TEQ_PROP_SCALAC_EVERY=1`, which the first campaign met once in 4,585 programs).
