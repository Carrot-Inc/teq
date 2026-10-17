# Brief: source-version-2 — the migration table's parser diagnostics, typer checks, for comprehensions and named tuples

The coordinator's ruling on source-version's split (2026-10-17): piece (i) landed as source-version (the model, the
scanning, the settings, the oracle cases and the harness, with the rows WithOperator, ExplicitContextBoundArgument,
ImplicitParamsWithoutUsing and UninitializedVars); piece (ii) is two parts, this one first: fifteen rows of dotty's
`MigrationVersion` table in four mechanism groups, with the default version's ripple handled as source-version
described. source-version-3 takes the grammar group after it.

## Today

source-version gave teq dotty's source version and its table: `src/source_version.rs` (`SourceVersion` on 3.10.0's
ordinals, `Migration`, 24 rows with each tag's thresholds, `Migration::outcome` as `report.errorOrMigrationWarning`,
`rewrite_notice` as `Message.rewriteNotice`), the parser's `Parser::migration` at the version where it stands and
the typer's `Worker::migration` at the unit's (`Ast::source_version`), `Category::Migration` rendered as
`migration warning`. Four rows report through it; the other twenty are read as before, which is one of three things
per row: silence, an unconditional diagnostic in teq's own words, or a grammar teq lacks (source-version-3's).

The oracle is in the tree: `tests/source-version.scala` (run by `tests/run_errors.sh`) runs each row's one-line
probe with no setting and at the versions its thresholds make distinct, both scalacs' captures pinned apart under
`tests/source-version/expected/<tag>/`, and `tests/source-version/pending.txt` lists the 76 runs teq does not give
as scalac yet, each with its row's group. A unit test (`source_version::tests::the_runs_cover_each_rows_boundaries`)
holds `runs.txt` to the table.

The default version is 3.8, and at 3.8 scalac 3.8.4 already warns for every row whose `warnFrom` is at most 3.4 and
whose `errorFrom` is later (`List[_]`, `private[this]`, `xs: _*`, `f _`, an alphanumeric method written infix, an
override of a `val` parameter), and refuses one (`NonNamedArgumentInJavaAnnotation`, an error from 3.6). teq gives
none of them today (docs/COMPATIBILITY.md's "Not yet"). Each row of this part therefore changes ordinary programs'
output at the default, as it changes scalac's.

## The aim

Each row reported as dotty reports it, at its sites, under the guards dotty puts around the call (several sites
report only past a version of their own, or only outside a pattern), with dotty's message whole (its rewrite notice
and its explanation lines included) at dotty's position, by `Migration::outcome` at the parser's or the unit's version,
and with dotty's message ID where the message has one (the `id` of `Parser::migration` and `Worker::migration`, a
constant of `warnings::id`: `-Wconf`'s `id` and `name` filters and `@nowarn` select it; the captures' headers name
the IDs, `E201` and `E204` among the rows of this part).
A row whose teq diagnostic exists in teq's words (NonLocalReturns) takes dotty's words and the gate; a row whose
lowering depends on the version takes the lowering too. Every run of the row in `pending.txt` passes and leaves the list.

### The parser's diagnostics (8 rows)

No grammar changes: each row is a site, a span and a message. Sites at 3.8.4 (`compiler/src/dotty/tools/dotc/`;
3.10.0's line numbers differ, its texts do not but where noted):

- **WildcardType** (3.4, future): `Parsers.simpleType` 2081–2089, a `_` type argument outside a match pattern
  (`!inMatchPattern`) and unless `-Xkind-projector:underscores`: "`_` is deprecated for wildcard arguments of types:
  use `?` instead" + `rewriteNotice(3.4-migration)`. teq: `parser/types.rs` 241 and 452.
- **VarargSpliceAscription** (3.4, future): `Parsers.ascription` 2623–2631, `x: _*` in arguments where
  `followingIsVararg`: "The syntax `x: _*` is no longer supported for vararg splices; use `x*` instead", at the `_`.
  teq: `parser/expr.rs` 475 (`spread_star_at`).
- **RemoveThisQualifier** (3.4, future): `Parsers.accessQualifierOpt` 3530–3542, `private[this]` / `protected[this]`,
  the three-line message with the reference link, spanning the modifier. teq: `parser/defs.rs` 178
  (`parse_access_qualifier`).
- **ImportWildcard** (future, future): `Parsers.wildcardSelector` 3986–3990, `_` for `*` in an import or export
  ("… a wildcard import; use `*` instead" with the clause's word). teq: `parser/mod.rs` `parse_import_selector`.
- **ImportRename** (future, future): `Parsers.namedSelector` 4005–4010, `a => b` for `a as b`. Same teq site.
- **GivenSyntax** (future, future), four sites: `contextBounds` 2358 (several context bounds not in braces),
  `givenDef`'s `adjustDefParams` 4563 (the old embedded colon), 4601 (an old-style abstract given, under
  `newSyntaxAllowed`), `withTemplate` 4833 (a given's body after `with`). teq: `parser/defs.rs` 1562–1693
  (`old_syntax`, `new_syntax`, `with_template`).
- **Scala2Implicits** (future, future): the parser's `expr` 2470 and `blockStatSeq` 5031 (an implicit lambda, which
  teq parses only where source-version-3's grammar lets it; the run that needs it is that part's), `paramMods` 3761
  (an `implicit` parameter clause), and the typer's `Checking.checkScala2Implicit` 523 (an implicit definition,
  conversion and class). The parameter clause's error is the one `ImplicitParamsWithoutUsing@future` waits for.
- **IdentifierDollars** (3.9, never; 3.10.0's row alone): 3.10.0's `Parsers.checkPatternName` 1319 and
  `Namer` 226–229 (`exempt`): `IllegalIdentifier`'s "The identifier `x$y` should not contain `$`, which is reserved
  for internal compiler use." on a definition's or a pattern's name, a warning from 3.9 and never an error; its
  expectation is 3.10.0's (`runs.txt`'s `=> 3.10.0`).

### The typer's checks (5 rows)

- **OverrideValParameter** (3.0, future): `RefChecks.checkOverride` (395) 628–632, rule 1.12: a member overriding a
  `ParamAccessor` that is not an inherited accessor (and not `tracked`): "cannot override val parameter
  value x in class A" at the member. teq has no such check; its overriding checks are `typer/check.rs`'s.
- **AlphanumericInfix** (3.4, future): `Checking.checkValidInfix` 1290–1318: an infix application of an
  alphanumeric name, not backquoted, whose method is not declared `infix`, not owned by a Scala 2 class, not compiled
  before 3.4 (its TASTy's minor under 4), and not followed by a lambda-like right operand (`infixOKSinceFollowedBy`);
  the kind by the mode (method, type, extractor) and the alternative it names. teq keeps `mods::INFIX`; the TASTy
  reader's minor of the method's file and Scala 2's owner flag are the inputs to find.
- **FunctionUnderscore** (3.4, future): `Migrations.typedAsFunction` 56–92: `f _`, the message's remedy by the
  expansion's arity (`functionPrefixSuffix`). teq: the eta-expansion of a postfix `_`.
- **NonNamedArgumentInJavaAnnotation** (3.6, 3.6): `Checking.checkNamedArgumentForJavaAnnotation` 1003–1029, a Java annotation's positional arguments
  but a lone one where the annotation has a `value` field (`annotationHasValueField`, which becomes `value = …`):
  an **error at the default**. teq: `typer/capture.rs` 60–139 (`java_annotation_class`, `type_java_annotation`).
- **NonLocalReturns** (3.2, future): `transform/NonLocalReturns.transformReturn` 99–103: teq warns today
  unconditionally with dotty's text (`typer/check.rs` 2005); the gate (silent below 3.2, migration warning at
  `future-migration`, error at `future`) and the place.

### The for comprehensions (1 row)

`Checking.checkIrrefutable` 1069–1128 came with app-blockers-3 (7a6d6e9f0) and reports through `Worker::migration`
since source-version's rebase: its message, remedy and rewrite notice, gated at 3.2, for a pattern definition
(ForComprehensionUncheckedPathDefs, whose runs all pass) and a generator (`typer/pat.rs` `check_irrefutable`).

- **ForComprehensionPatternWithoutCase** (3.2, 3.4): the generator's mode by the parser's version
  (`Parsers.generatorRest` 3123–3126: `FilterAlways` with `case`; else `Check` from 3.4, `CheckAndFilter` at 3.2 and
  3.3, `FilterNow` below 3.2). The lowering changes what the program does: below 3.4 a refutable pattern filters
  (`withFilter`), from 3.4 it is checked, a `MatchError` at run time under `3.4-migration` (`forpat_run@*`, the
  interpreted runs; `forpat_run@3.1` and `@3.3` pending). teq filters an explicit `case` generator only
  (`parser/expr.rs` `Enumerator::CaseGen`) and checks the rest at every version.

### The named tuples (1 row and its gate)

- `enablesNamedTuples` (3.7): `Parsers` 681 (`(a = 1)` as a named tuple argument) and 2245 (a named tuple type),
  `Typer` 857; below 3.7 `(a = 1, b = 2)` is an assignment's argument (`namedtuple_old@3.6`: "Not found: a").
- **AmbiguousNamedTupleSyntax** (3.6, future): `Parsers.migrateInfixOp` 1195–1205 (an infix operator whose right
  operand reads as named arguments, `DeprecatedInfixNamedArgumentSyntax`, the named tuple's operators exempt) and
  `Typer` 3766–3780 (`DeprecatedAssignmentSyntax`, a migration warning where the parenthesized assignment types).

## The ripple

Every row of the first two groups but the future-only ones and IdentifierDollars changes the default's output. The
repository's output-compared suites (`tests/cases`, which `run_interp.sh` and `run_jvm.sh` share, `tests/errors`'
`.check` files, the warning suites' captures, scala3-typing's warn suite held to scalac 3.8.4's captured diagnostics
whole, `warn-oracle.jsonl`) hold the constructs: source-version measured `List[_]` in 66 case files
and 6 of other suites, `private[this]` in 3, `xs: _*` in 1, `f _` in about 15 and 1; AlphanumericInfix,
OverrideValParameter and the Java annotations are to be measured by the design pass. Each warned site is silenced as
scalac's warns there too (`@nowarn` on the definition, or `// teq: --wconf msg=…:s` for a case about the construct:
`tests/cases/nonlocal_return_lambda.scala`, `var_default_init.scala`, `with_type_intersection.scala`), never by
rewriting a case's construct away; the scala3-typing harness re-recorded with each moved row explained. The gate's
project and application lines (lila, the application's `app-api` against master's diagnostics) will show the new
warnings where scalac gives them: the part names the counts in its report.

## Out of scope (task lines if met)

The grammar group (source-version-3); XmlLiteral (teq has no XML); `-rewrite` and its patches; the findings of
source-version (an import of `scala.language.<unknown>` unresolved; the default ExplicitContextBoundArgument run's
wording); 3.10.0's behaviours beyond its table (the migration's).

## Cases

The runs of `tests/source-version/pending.txt` of the fifteen rows (and `ImplicitParamsWithoutUsing@future*`,
`namedtuple_old@3.6`, `forpat_run@3.1` and `@3.3`) pass and leave the list; each row gains what its one-liner does not show: every
site of GivenSyntax and Scala2Implicits, WildcardType inside a match pattern (silent) and under kind-projector,
AlphanumericInfix's exemptions (backquoted, an operator, declared `infix`, a Scala 2 method, a TASTy before 3.4, a
lambda after it) and kinds (type, extractor), OverrideValParameter's tracked and inherited accessor, a Java
annotation with a `value` field and a lone argument, the for comprehension's
three modes run under the interpreter, JavaScript and the JVM. Each new probe captured by both scalacs
(`--capture 3.8.4`, `--capture 3.10.0`), held to 3.8.4's but where 3.8.4 cannot show the row.

## Rules

- The Dotty-approach rule: every site validated against dotty's source at 3.8.4 (3.10.0 for IdentifierDollars), in
  dotty's shape, its guard as dotty writes it; a shortcut that passes the probes is a finding.
- Bounded shells (6 minutes), `cargo build --profile fast` and `TEQ=target/fast/teq` for iteration, release builds
  for measurements; never `git stash`; commits with short verb-first subjects naming dotty's file and method, no
  body, no trailers; the report at `~/reports/source-version-2.md`.
- Before a push: the touched suites on the release build (parser, errors with the source runs, cases, interp, jvm,
  check-watch, lsp where the parser or the reporting changes, scala3-typing re-recorded with its rows explained), and
  the instruction pair against master on the five programs (depth_1, gview_1, core-only, realistic-api,
  realistic-frontend; within 0.5% each; a check on a hot path a cost to measure, never to assume).
- A scalac 3.10.0 run uses `COURSIER_CACHE=$HOME/.cache/coursier-310` where std-jar-version's binary is not yet the
  one under test.
- Nothing of the application in the tree.
