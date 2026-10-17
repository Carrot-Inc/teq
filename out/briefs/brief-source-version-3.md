# Brief: source-version-3 — the migration table's grammar group: Scala 2's syntax under `3.0-migration`, the ascription after a pattern, the implicit lambda

The coordinator's ruling on source-version's split (2026-10-17): piece (ii) is two parts; source-version-2 takes the
parser's diagnostics, the typer's checks, the for comprehensions and the named tuples; this part takes the rows whose
old syntax teq does not parse, each grammar before its gate: Scala2to3, AscriptionAfterPattern and
ParameterEnclosedByParenthesis (with Scala2Implicits' implicit-lambda sites, which need the last's grammar).

## Today

source-version gave teq dotty's source version, the table (`src/source_version.rs`, `Migration::outcome` as
`report.errorOrMigrationWarning`), the parser's and the typer's `migration` reports, and of Scala2to3 the scanner's
site: under `3.0-migration` `given` and `enum` are identifiers with dotty's migration warning, in an interpolation too
(`Scanners.toToken`'s `handleMigration`, `getInterpolatedIdentRest`), and a `3.0-migration` setting turns indentation
off (`indentSyntax`, `isEndMarker`). Everything else `3.0-migration` means to dotty teq does not do: Scala 2's
syntax is refused as at any version, and the errors the version downgrades stay errors.

`tests/source-version/pending.txt` holds this part's runs: `Scala2to3`, `Scala2to3@3.0-migration`, `Scala2to3@3.0`
(procedure syntax), `AscriptionAfterPattern*` (5) and `ParameterEnclosedByParenthesis*` (4).

## The aim

### Scala2to3 (3.0, 3.0): a migration warning under `3.0-migration`, an error from 3.0

Its `errorFrom` is its `warnFrom`, so every site is a migration warning at `3.0-migration` and an error at every
later version: a site the parser reaches under `migrateTo3` alone is a migration only, a site of the typer that every
version reaches is today's error downgraded under `3.0-migration`. 18 reporting sites at 3.8.4 (26 references with
their patch guards), `compiler/src/dotty/tools/dotc/`:

- the parser, reached under `3.0-migration` (each a grammar teq lacks, built first): `Parsers.convertToParams` 484
  (a lambda's parameter not in parentheses, `x: Int => …`), `argumentStart` 1566–1575 (a `{` on the next line takes
  the argument, "This opening brace will start a new statement in Scala 3."), `contextBounds` 2361–2365 (view
  bounds, `<%`), `expr1` 2519 (`do <body> while <cond>`), `ascription` 2638 (`_*` not the last argument),
  `defDefOrDcl` 4160–4170 (procedure syntax, `def f() { … }`, with the constructor's and the definition's
  `newLineOptWhenFollowedBy(LBRACE)` 4183, 4204), `template` 4752–4758 (`extends` without a parent);
- the parser at every version: `digits`'s symbol literal 1468–1477 (`'name`, unless `language.deprecated.symbolLiterals`),
  an error at the default teq gives in its own words;
- the scanner: `isLeadingInfixOperator` 465–483 (a line starting with an operator, "it is now treated as a
  continuation…"), and the layout `migrateTo3` changes: `isContinuing` false (594), the token sets of Scala 2
  (`canStartStatTokens2`, `canStartExprTokens2`, `DO` starting an expression, 1243–1246), `Parsers` 2392 (an
  expression goes on past a line end only under the new syntax);
- the typer, at every version (an error teq gives today, where it gives one, downgraded to a migration warning under
  `3.0-migration`): `Checking.checkWildcardApply` 136 (an unreducible application of a wildcard),
  `checkRealizable` 227, `Checking` 833 (a private type in a public signature, `NotPrivate`),
  `Migrations.kindProjectorQMark` 54 (`?` as a type name), `typedAsFunction` 67 (`OnlyFunctionsCanBeFollowedByUnderscore`),
  `RefChecks` 926 (an illegal implementation, `addDecls`) and 944 (a case class's variant inheritance),
  `Typer` 506 (`AmbiguousReference` between an inherited and an outer definition, `sameTermOrType`).

The part sizes each site before building it, as source-version did the rows: the grammar of procedure syntax, of
`do`-while and of view bounds is modest; the scanner's Scala 2 layout touches every line break under
`3.0-migration` and is measured against scalac on the source-version probes and on Scala 2 sources (lila's or a
library's), never by a special case; the typer's downgrades need teq's errors at those places first, a site teq does
not check being a finding with its program, not this part's to add.

### AscriptionAfterPattern (3.3, future)

`Parsers.pattern1` 3325–3340: an ascription after a pattern other than a variable or a number literal
(`case Some(y): Option[Int] =>`), parsed as `Typed(pattern, type)` and warned from 3.3 (a warning at the default), an
error at `future`. teq refuses the grammar at every version (`parser/pat.rs`): the pattern's ascription first (its
typing as dotty's `typedTyped` of a pattern), then the gate.

### ParameterEnclosedByParenthesis (future, future) and Scala2Implicits' lambdas

`Parsers.funParams` 2730–2746: a block's lambda whose parameter is not in parentheses (`{ x: Int => x }`) and the
implicit lambda (`{ implicit x: Int => x }`, `implicit x => …`), which `Parsers.expr` 2470 and `blockStatSeq` 5031
read (`closure(…, modifiers(BitSet(IMPLICIT)))`) and report as Scala2Implicits (future, future). teq parses neither
(`implicit` before a lambda is refused). The probe's `future-migration` run gives two warnings, the two rows', which
both go.

## Out of scope (task lines if met)

`-rewrite` and its patches; `-indent` and `-no-indent` (`-indent` keeps indentation under `3.0-migration`; teq reads
neither); XML literals; Scala 2's semantics beyond the table's sites (`-Xsource`, Scala 2 pickles' own reading).

## Cases

The runs of `pending.txt` above pass and leave the list. Each site gains a probe in `tests/source-version/probes`
captured by both scalacs at `3.0-migration` and `3.0` (or its row's versions), held to 3.8.4's: procedure syntax with
a brace and with `: Unit`, a constructor's, `do`-while, view bounds, a lambda's unparenthesized parameter, a brace on
the next line taking the argument and starting a statement at `3.0`, `_*` not last, `extends {`, a symbol literal
with and without the feature import, a leading infix operator at a line's start and inside parentheses, and each of
the typer's downgrades teq checks; an ascribed constructor pattern at 3.2, 3.3, `future-migration` and `future`, run
for its result; the implicit lambda in a block and as an argument, with and without parentheses; a Scala 2 source
of some size (a file of a published Scala 2.13 library's, its licence permitting) compiled under `-source:3.0-migration`
by both, the warnings' places and counts compared.

## Rules

- The Dotty-approach rule: every site validated against dotty's source at 3.8.4, in dotty's shape, its guard
  (`migrateTo3`, a feature, a version) as dotty writes it; a grammar built for the probe alone is a finding.
- Bounded shells (6 minutes), `cargo build --profile fast` and `TEQ=target/fast/teq` for iteration, release builds
  for measurements; never `git stash`; commits with short verb-first subjects naming dotty's file and method, no
  body, no trailers; the report at `~/reports/source-version-3.md`.
- Before a push: the touched suites on the release build (parser, its fixtures and mutation gate as the grammar
  grows, errors with the source runs, cases, interp, jvm, check-watch, lsp, scala3-typing re-recorded with its rows
  explained), and the instruction pair against master on the five programs (depth_1, gview_1, core-only,
  realistic-api, realistic-frontend; within 0.5% each): the default version's parsing must not pay for
  `3.0-migration`'s grammar.
- Nothing of the application in the tree.
