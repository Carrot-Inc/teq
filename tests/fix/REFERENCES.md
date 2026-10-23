# The references of `teq fix`'s rules and of these cases

`tests/fix.scala` runs every case of this directory (its header says how); docs/FIX.md describes the rules. A
rule teq ships is ported from its repository at the revision below, its file keeping the repository's notice;
each case but teq's own (below) is a fixture of that repository, taken as it is but for the adaptations listed, so
that a case's expectations are the reference's own. NOTICE carries the five repositories, and scalameta, whose
parser's leading-comment scan the planner ports.

## The repositories

| Repository | Revision | Licence | Taken here | Taken by the next part (fix-semantic) |
|---|---|---|---|---|
| [scalacenter/scalafix](https://github.com/scalacenter/scalafix) | `41cf7983a19a0453be78d1fe9863d77470ca6dec` | BSD 3-Clause, `LICENSE.md` | `DisableSyntax` (`scalafix-rules/src/main/scala/scalafix/internal/rule/DisableSyntax.scala`, `DisableSyntaxConfig.scala`); the fixtures of `scalafix-tests/input/src/main/scala/test/disableSyntax`, `scala-3/test/disableSyntax` and `scala/test/escapeHatch`; the test rules `NoDummy` and `NoNull` (`scalafix-tests/integration/src/main/scala/scalafix/test`), ported as `support/`; the semantics the driver follows: `PatchInternals`, `ImportPatchOps` (an added import's place, ported in `src/fix/plan.rs`), `EscapeHatch`, `RuleDiagnostic`, `LintConfig`, `FilterMatcher` | |
| [xuwei-k/scalafix-rules](https://github.com/xuwei-k/scalafix-rules) | `995edb7a922724bc680158bc1cca61d568e72c88` | MIT, `LICENSE.txt` | `CatsImplicitsImport` (`rules/src/main/scala/fix/CatsImplicitsImport.scala`) and its fixture | `OptionGetWarn` |
| [ghostbuster91/scalafix-unified](https://github.com/ghostbuster91/scalafix-unified) | `6563e96c9f0b26175c226b758c752ce1123249ae` | Apache 2.0, as its `build.sbt` declares (`licenses := List("Apache-2.0" -> ...)`); the repository has no licence file | `EmptyCollectionsUnified` (`rules/src/main/scala/fix/EmptyCollectionsUnified.scala`) and its input and output fixtures | |
| [typelevel/typelevel-scalafix](https://github.com/typelevel/typelevel-scalafix) | `f5bc61f93fd290cd83f27bf277bb740f2a6a808a` | Apache 2.0, `LICENSE` | `TypelevelAs` (`modules/cats/rules/src/main/scala/org/typelevel/fix/As.scala`) and its fixture | `TypelevelHttp4sLiteralsSyntax` |
| [tanin47/scalafix-forbidden-symbol](https://github.com/tanin47/scalafix-forbidden-symbol) | `813a206a4218dc86b52718077fad5d82f2c3d4ed` | MIT, `LICENSE` | | `ForbiddenSymbol` |
| [scalameta/scalameta](https://github.com/scalameta/scalameta) | `v4.17.4`, `51efeb6b0fc0c8e7c7232167fa99f7d2ee8403b1` (scalafix `41cf798`'s `scalametaV`) | BSD 3-Clause, `LICENSE.md` | the leading-comment scan `begComment` (`scalameta/parsers/shared/src/main/scala/scala/meta/internal/parsers/ScalametaParser.scala`) and `canEndStat` (`ScannerTokens.scala`), ported in `src/fix/plan.rs` for an added import's place | |

## The cases

A case is a directory `<Rule>/<Fixture>/` holding `input.scala` (the fixture, its header comment the
configuration as scalafix's testkit reads it), `output.scala` where the rule rewrites (the fixture's output
file), `expected.txt` (the lints as `teq compiler fix` prints them) and, where the fixture needs libraries,
`jars` (keys of `tests/support/jars.sh`). The expectations are the fixture's assertions: a `// assert: <id>`
comment names a lint of that id on its line, a `/* assert: <id>` comment with a caret line and a message
also its column, its width and its message. The suite checks `expected.txt` against them, so that the
recorded lints are the reference's and no more; a case's exit is 1 where an error-severity lint remains, 0
otherwise, and under `--check` 1 where there is a lint or an edit.

| Case | Fixture (under the repository's root) | Adaptation |
|---|---|---|
| `DisableSyntax/DisableSyntaxBase` | `scalafix-tests/input/src/main/scala/test/disableSyntax/DisableSyntaxBase.scala` | `override` removed from `def finalize()` (defect 1) |
| `DisableSyntax/DisableSyntaxFinalVal` | `.../test/disableSyntax/DisableSyntaxFinalVal.scala` | |
| `DisableSyntax/DisableSyntaxInstanceOf` | `.../test/disableSyntax/DisableSyntaxInstanceOf.scala` | |
| `DisableSyntax/DisableSyntaxMoreRules` | `.../test/disableSyntax/DisableSyntaxMoreRules.scala` | the statements `1.==(2)` and `1.!=(2)` with their assertions removed (defect 2); `1 == 2` and `1 != 2` stay |
| `DisableSyntax/DisableSyntaxNoValPatterns` | `.../test/disableSyntax/DisableSyntaxNoValPatterns.scala` | |
| `DisableSyntax/NoFinalize` | `.../test/disableSyntax/NoFinalize.scala` | `override` removed from every `finalize` and the caret of the multi-line assertion moved with its name (defect 1); `DisableSyntax`'s `FinalizeMatcher` reads no modifier |
| `DisableSyntax/NoTabs` | `.../test/disableSyntax/NoTabs.scala` | |
| `DisableSyntax/MatchableDisableSyntax` | `scalafix-tests/input/src/main/scala-3/test/disableSyntax/MatchableDisableSyntax.scala` | |
| `DisableSyntax/SpecificDisableSyntax` | `scalafix-tests/input/src/main/scala-3/test/disableSyntax/SpecificDisableSyntax.scala` | |
| `Suppression/<name>` for `AnchorAnnotationMixed`, `AnchorDescription`, `AnchorDoubleOn`, `AnchorEOF`, `AnchorIntervals`, `AnchorOverlapping`, `AnchorRange`, `AnchorUnused`, `AnchorWildcard`, `AnnotationFullyQualified`, `AnnotationScopes`, `AnnotationUnused`, `AnnotationWithNonLiteralValue`, `Issue507` | `scalafix-tests/input/src/main/scala/test/escapeHatch/<name>.scala` | the rules `"class:scalafix.test.NoDummy"` and `"class:scalafix.test.NoNull"` named as `"file:../../support/NoDummy.scala"` and `"file:../../support/NoNull.scala"`, the test rules ported to teq's API (scalafix loads them from its test class path); `AnnotationScopes`: the class `DummyClass_2` whose primary constructor carries the annotation removed with its two assertions (defect 3) |
| `CatsImplicitsImport/CatsImplicitsImportTest` | xuwei-k: `input/src/main/scala/fix/CatsImplicitsImportTest.scala` | cats 2.13.0 (`jars`) where the reference's build names its own cats |
| `TypelevelAs/AsTests` | typelevel: `modules/cats/input/src/main/scala/fix/AsTests.scala` | likewise |
| `EmptyCollectionsUnified/EmptyCollectionsUnified` | scalafix-unified: `input/src/main/scala/test/EmptyCollectionsUnified.scala` and `output/src/main/scala/test/EmptyCollectionsUnified.scala` | the `val k` whose pattern is `::(_, Nil)` removed from both (defect 4); `1 :: Nil` and `::(1, Nil)` rewritten to `List.empty` in the output, which the repository records under scalameta 4.4.32 (its build's), where an argument's parent is its application: under 4.17 it is a `Term.ArgClause`, so the rule's `::` cases match nothing, and scalafix 0.14.7 with the rule's 0.0.8 jar rewrites both |

## teq's own cases

- `NoUnusedImports/<name>`: the programs of `tests/warnings/edits/` (scalac 3.8.4's `-Wunused:imports` cases),
  each with the rule's header, its `output.scala` what scalafix-cli 0.14.7 leaves of it with a rule removing the same
  importees through `removeImportee` (`ImportPatchOps`' token removals), with dotty's deletion of an import's line
  (`editPosAt`'s blank line, in `one` and `blank_lines`): what `tests/warnings/expected/edits/<name>.scala` records
  scalac's `-rewrite` to leave but in `block`, `comments` and `nested_comment` (`hiding`: both imports gone, as one run
  of the rule removes what scalac's removes in two;
  `crlf`: the kept selector's import ending in the line's CRLF, where scalac's `editPosAt` leaves an LF, docs/FIX.md); `macro_expansion` (from `tests/warnings/probes/macro_expansion`), a file where a derived macro
  expands, its definitions beside it (`En.scala`), its importee the warning `NoUnusedImports.unverified`; `after_rewrite`, the second round; `after_rewrite_other_file`, the second round over a file the first
  did not edit (`Other.scala`), its lint and its suppression the first round's.
- `NoUnusedImports/crlf_one_kept`: a CRLF file whose clause keeps one selector, written as that selector's import with
  its line's CRLF (the review's probe of `380470c41`); `rename_kept` (a clause left with one rename keeps its braces)
  and `comment_after_comma` (an importee after a line comment that follows the comma before it: scalafix-cli 0.14.7
  removes it without that comma, a text that does not parse, and teq writes dotty's `checkImports` edit, as
  `comment_before_last` does for the last importee), from scalafix's `ImportPatchOps` at v0.14.7; `trailing_comma_kept`
  (the review of `252f52716`), Scala's trailing comma kept with scalafix's edit, its output scalafix-core 0.14.9's.
- `Merge/<name>`: the planner's merge of the rules' edits on one token (`PatchInternals.merge`) and the line break a
  deleted import's line loses, beside a rule that sorts the leading imports (`support/SortBlock.scala`): an unused
  import in the middle, the last, the first after a code line, a blank line and a comment, a clause's importee;
  `DedupSort`, a token one rule removes (`support/DedupImports.scala`) and the sorting rule adds to. `SharedLints`
  (the review of `0c12b7579`), a build's case: a file two configuration files' configurations share, one of them
  revisited by round two, the other's lint on it printed by the first run as by the second; `SharedState` (the review
  of `2f7c9255a`), a project rule whose lint depends on its visits, the revisited job's shared file joined with that
  job's own round-one results.
- `Suppression/AtomicCommentsBetween`, `AtomicCommentsOneKept` and `AtomicCommentsApart` (the review of `4fd7f3293`)
  as scalafix-cli 0.14.7 leaves them with a rule removing `List` atomically: the removal touches no anchor and is
  applied, the anchors suppressing nothing; in `AtomicCommentsApart` `List` goes without a comma, which teq writes as
  dotty's `checkImports` edit.
- `Driver/<name>`: the driver's own cases, each a probe of the review of `380470c41`: `StalePatch` (a patch made from
  another document refused, `refused.txt`), `Extractors` (the review of `e9bffbf79`: `Defn.Var`'s legacy `rhs`, Some and None, and the formats and value
  `Lit.Double`, `Lit.Float` and `Lit.String` give as `String`s, as scalameta 4.17.4's), `LiteralUnits` (the review of `2f7c9255a`: a string's value a `String`, the literals' UTF-16
  units as scalameta 4.17.4 reads them, a lone surrogate's in a string, a part and a char), `LiteralSuffixes` (a literal's suffix in its tree's span and its
  ancestors'), `TieOrder` (two patches at one start, refused in dotty's order), `CRComment` (a line comment ending at a
  lone CR, `Lone.scala`); their rules are in `support/`.
- The probes of the review of `a24540c28`: `NoUnusedImports/macro_expansion_block` (`macro_expansion` in a package
  block), `Driver/RetainedToken` (a CRLF file's multiline string with an LF alone, text added beside it; rule
  `support/MarkStrings.scala`), `Driver/TemplateSpans` (templates with their braces, an empty one replaced; rule
  `support/Templates.scala`), `DisableSyntax/HigherKindedVariance` (`F[+A]`'s variance, nested ones too),
  `DisableSyntax/KeywordsNotAList` and `RegexNotAList` (a string where a list is read, refused),
  `Driver/LintFilterRegex` (a filter's regex that does not compile, refused) and `Driver/LintFilterDefaults` (a
  filter object naming excludes alone); the verb's `excluded file`, `no source` and `relative config` checks.
- The probes of the review of `86884ea36`: `EmptyCollectionsUnified/Script` (a script edited as written,
  `input.sc`), `Suppression/AtomicImport` (an atomic import addition suppressed; rule `support/AddImmutable.scala`),
  `DisableSyntax/RegexQuotedSpaces` (a quoted pattern's spaces kept), `DisableSyntax/BackquotedCasts` (backquoted
  `asInstanceOf`, `isInstanceOf` and `Matchable`), `DisableSyntax/HigherKindedAnnotated` (`F[@ann +A]`),
  `Driver/LambdaBinders` and `Driver/LambdaBindersI13541` (dotty's `tests/pos/i13541.scala` at 3.8.4, its
  assertion comments added; rule `support/Params.scala`), `DisableSyntax/LowercaseSectionTypo` (refused); the
  verb's `generated untouched` and `generated not named` checks; `tests/errors/hk_param_context_bound.scala`.
- The probes of the review of `340567fdd`, the part's fix-up: `Driver/BinderAnnotationUnknown` (a binder's unknown
  annotation, refused) with `tests/errors/binder_annotations.scala` (E006, E007); `Driver/PolyContextBounds` (a
  polymorphic function's context bounds, a type lambda's `_`) with `tests/errors/type_param_owners.scala` (a type
  lambda's bounds refused, E214 with `as`); `DisableSyntax/AnnotatedTypeEquality` and `Driver/TypeAnnotations`
  (rule `support/Annotations.scala`); `Driver/ParamClauseSpans` (rule `support/FillClauses.scala`);
  `Suppression/AtomicImportWritten`; `Driver/MadeLambdas` (rule `support/Lambdas.scala`).
- The probes of the review of `16412b2e9`, the second fix-up: `Driver/AnnotatedModifiers` (a soft modifier after an
  annotation without arguments; rule `support/Modifiers.scala`); the compiler's side in
  `tests/errors/binder_annotation_checks.scala`, `tests/cases/poly_context_bound_names.scala` and
  `tests/warnings/policy/nowarn_binder.scala`.
- The probes of the review of `50e3a1b38`, the third fix-up, the compiler's side alone: `tests/errors/binder_annotation_args.scala`,
  `binder_java_annotation_args.scala`, `binder_completion_cycle.scala`, `context_bound_evidence_name.scala`,
  `tests/cases/poly_evidence_tree_edges.scala`, `tests/warnings/policy/nowarn_binder_context.scala` and
  `nowarn_binder_werror.scala`.
- `DisableSyntax/PlaceholderFunctions`: placeholder functions (`_ + 1`, `(_: Int) != 1`, `_ * _`) under
  `noUniversalEquality`, their `_`s scalameta's `Term.Placeholder`s and their functions `Term.AnonymousFunction`s,
  which a rule matching an infix application's operand reads as terms.
- The cases of fix-speed (the rules over several workers, the guarded checks): `Driver/TokenContract` (every token's
  kind, offsets, text, `is`, keyword and trivia classes and value, over an input of every token class teq lexes; rule
  `support/DumpTokens.scala`), `Driver/TreeContract` (the design pass's probe of the tree in both orders of access and
  after another document; rule `support/InspectTree.scala`), `Driver/FieldContract` (every kind's named fields through
  its accessors, an empty one and a missing one's exception; rule `support/DumpFields.scala`), each recorded on the
  binary before the part's changes; `DisableSyntax/EndMarkerKeywords` (an end marker's keyword name a keyword's token,
  as scalameta's tokenizer scans it, reported as scalafix's `Keyword` extractor reports it, `end given` not: from
  scalafix's `DisableSyntaxConfig.scala` at `41cf7983`, `Keyword.unapply` and `kwSet`); `DisableSyntax/FinalizeUnconditional`
  and `FinalizeDefaultArgs` (`FinalizeMatcher` read no setting, after `DefaultMatcher`'s cases: from scalafix's
  `DisableSyntax.scala` at `41cf7983`, `checkTree`); `DisableSyntax/GuardsAllOff`, `GuardsAllOn`, `GuardsApplication` and
  `GuardsMixed` (one input with every check under four configurations, recorded by the rule before its settings were
  asked first); `Driver/ReceiverIdentity` (`collect` and `traverse` visiting their receiver as itself, the root's and an
  inner node's: the push pass of `becf46896`, its Root and Identity probes; rule `support/ReceiverIdentity.scala`). The
  `parallel` cases of `tests/fix.scala` generate their corpus. The `typing` cases (fix-parallel-typing) generate their
  witness corpus too, and read the design pass's programs from `support/typing/`: `counter/` (a macro library whose run
  changes state other runs share, read from its products, its users and an edit for round two; `bad/ZBad.scala` a type
  error), `marks/` (a wildcard, a hiding selector, a given and a name marked by readers in other files), `withheld/`,
  `records/` (an inline expansion and overloads), `blocks/` (CRLF, braced packages), `phase-errors/`, `round2/`; their
  rule `support/Print.scala` prints its configuration.
- The cases of app-fix-rules (the tree laid out as scalameta 4.17.0's): `Driver/LayoutPackages`, `LayoutTemplates`,
  `LayoutClauses`, `LayoutDefinitions`, `LayoutTerms` and `LayoutPatternsAndTypes`, each with the layout
  `support/MetaDump.scala` prints of its input under scalameta 4.17.0 (`scalameta.txt`), read against teq's;
  `DisableSyntax/ScalametaLayout` and `ExtensionFinalize` (the rule's checks over that layout), their lints those
  scalafix-cli 0.14.7 reports over the same inputs; `Driver/RuleMetadata` (`Rule`'s `isLinter`, `isRewrite` and
  `isExperimental`, a rule's own and DisableSyntax's, a linter as its source declares; rule `support/Metadata.scala`).
  `Driver/CheckPositions`: `--check` prints round one's lints at the input's lines (scalafix's `--check`), a rewrite
  its last round's at the lines of the file it writes.
  `Driver/ImporteeExtractors`: `Importee.Given(tpe)` and `GivenAll()` as scalameta's extractors (rule
  `support/Importees.scala`).
  `Driver/StablePatterns`: a stable identifier as a case's pattern, its `Term.Name` or `Term.Select` a `Pat` as
  scalameta's (rule `support/StablePatterns.scala`).
  `Driver/LayoutParenTerms`, `LayoutParenPatterns` and `LayoutParenTypes`: a term, a pattern and a type in parentheses
  at every grammar position, against scalameta 4.17.4's layout; the ledger of the parser's parentheses (the pass over
  `2865c7af2`) added a parameter's type in every clause, a dependent function type's named parameter, an argument's
  ascription, a guard's condition and a colon argument's body. `Driver/ReplaceLambdaParam` (rule
  `support/ReplaceLambdaParam.scala`): an ordinary and a colon argument's lambda parameter of a parenthesized type
  replaced whole, scalafix-core 0.14.9's output. The review of `e0d4a9c4f`: `Driver/LayoutAnnotateImplicit` (each
  written annotation ascription its own `Term.Annotate`, an implicit lambda's `Mod.Implicit`s, against scalameta's
  layout), `ImplicitLambdaParams` (the clause's and the parameter's syntax and modifiers, rule
  `support/ImplicitParams.scala`), `ReplaceImplicitParam` and `UnwrapNestedAnnotate` (rules
  `support/ReplaceImplicitParam.scala`, `support/UnwrapAnnotate.scala`, scalafix-core 0.14.9's outputs).
  `Suppression/OkLastStatement`: an ok anchor trailing the last statement of a colon body is that statement's
  `endComment` alone, scalameta 4.17.0's parser sharing a trailing comment up to a block (`atPosWithBody`).
  `Driver/AnonymousParams`: an anonymous parameter's name, scalameta's `Name.Anonymous` (rule `support/ParamNames.scala`).
  The verb's `include`, `include error` and `include setting` checks: the configuration read as typesafe-config
  1.4.6 reads it under metaconfig 0.18.6, whose results over the cases of src/fix/config.rs's tests were recorded by
  running both over the same files.

## Fixtures not taken

- `scala-2/test/disableSyntax/SpecificDisableSyntax.scala`: Scala 2's dialect.
- `test/disableSyntax/NoXml.scala`: teq reads no XML literal (a file with one fails to parse), and scalac reads
  one only with scala-xml; `DisableSyntax.noXml` is decoded and reports nothing (docs/FIX.md).
- `test/escapeHatch/AnchorExpression`, `AnchorMultipleRules`, `AnnotationMultipleRules`,
  `AnnotationPrefixedRules`, `AnnotationWildcard`, `Issue694`: they run scalafix's semantic `Disable` rule,
  which teq does not ship.
- The fixtures of the next part's rules (OptionGetWarn, ForbiddenSymbol, TypelevelHttp4sLiteralsSyntax).

## The defects of teq the fixtures met

scalac 3.8.4 accepts each fixture as it stands; teq rejects four constructs in them, each held by a program in
`tests/pending-cases/` until it is fixed, when the adaptation above is undone:

1. A `finalize` overriding `java.lang.Object`'s ("method finalize overrides nothing"):
   `tests/pending-cases/object_finalize_override.scala`.
2. `==` and `!=` selected and applied (`1.==(2)`): `tests/pending-cases/primitive_eq_selected.scala`.
3. An annotation on a primary constructor (`class C @A (x: Int)`):
   `tests/pending-cases/annotated_primary_constructor.scala`.
4. `::` as an extractor in a pattern (`case ::(_, Nil)`), whose recovery also fails the later pattern
   definitions of the fixture: `tests/pending-cases/pattern_cons_extractor.scala`.
