# teq fix: lint and rewrite rules

`teq fix` (docs/CLI.md) runs scalafix-shaped rules over a build's sources with no sbt and no JVM. A rule is Scala,
run by teq's interpreter against the API of the std's package `teq.fix`, shaped after scalafix's `scalafix.v1` so
that a scalafix rule reads alike and ports with local changes; it reads the trees and tokens of the files the
compiler parsed and typed in the same process, and returns patches and lints, which `teq fix` plans, checks and
applies. A program teq does not type is exit 2 before any rule runs, the syntactic rules among them.

## Configuration

`.scalafix.conf` beside `teq.lock` (or the file `--config` names), read by teq: the HOCON that scalafix's
configurations use. `key = value`, `key: value` and `key { ... }`, dotted keys (`DisableSyntax.noVars = true`),
objects merged as HOCON merges them, lists with or without commas and over lines, quoted and bare strings (a quoted
one's characters as written, its spaces among them), adjacent strings joined with what stands between them
(`"print""ln"`), numbers, booleans, `null`, `#` and `//` comments. Anything else of HOCON (`include`,
`${...}`, `+=`, `"""`) is an error at its place (exit 2), never skipped.

```
rules = [DisableSyntax, NoUnusedImports, "file:fix/NoToString.scala"]
DisableSyntax.noVars = true
DisableSyntax.regex = [{ id = offensive, pattern = "[Pp]imp", message = "Please consider a less offensive word" }]
lint.error.includes = ".*"
```

- `rules` (or `rule`): a rule's name, or `file:<path>`, a Scala file or a directory of them relative to the
  configuration's directory, every concrete class there extending `teq.fix.Rule` with a constructor of no
  arguments a rule under its `name`. `--rules A,B` replaces the list. A name of no rule is an error naming it.
  A `class:<name>` entry is not read: it names no rule (exit 2).
- A rule's settings are under its name, or a section its decoding reads instead (`disableSyntax`). A key of either
  that the rule's decoding does not read is an error at its place (`DisableSyntax.noVar: no such setting`), as
  metaconfig's unknown field is; the settings of a rule the run does not configure are ignored, as scalafix ignores
  them.
- `lint.error`, `lint.warning`, `lint.info` and `lint.ignore`: scalafix's `LintConfig`, each a `FilterMatcher`
  (a regex, a list of them standing for their alternation, or `{ includes, excludes }`, a regex each, an object
  naming none including everything and excluding nothing) found in a lint's full id, `<rule>.<category>` (or
  `<rule>` for an empty category); the error filter first, then the warning one, then the info one, else the
  lint's own severity; `lint.ignore` drops a lint. A regex that does not compile, or a value of another kind, is an
  error at its place before any rule runs (exit 2), as `FilterMatcher.reader` decodes.
- A setting a rule reads as a list (`DisableSyntax.keywords`, `regex`) that is no list is an error at its place
  (exit 2), as metaconfig's collection decoders have it: `Conf.asList`, `asStrings`.

## The rule API

```scala
import teq.fix.*

final class NoPrintln(message: String) extends SyntacticRule("NoPrintln"):
  def this() = this("use the logger")

  override def withConfiguration(c: Configuration): Configured[Rule] =
    c.conf.section("NoPrintln").string("message", message).map(new NoPrintln(_))

  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect {
      case t @ Term.Apply(Term.Name("println"), List(arg)) =>
        Patch.lint(Diagnostic("call", message, t.pos)) + Patch.replaceTree(t, "log.info(" + arg.syntax + ")").atomic
    }.asPatch
```

- `SyntacticRule(name)` with `fix(using doc: SyntacticDocument): Patch`, `SemanticRule(name)` with
  `fix(using doc: SemanticDocument): Patch`; `withConfiguration` returns the configured rule or `Configured.NotOk`.
- `SyntacticDocument`: `input` (its `text` and `path`), `tree` (the `Source`), `tokens`.
  `SemanticDocument` adds the typer's answers: `unusedImports`, the importees the typer's unused-import analysis
  reports (`-Wunused:imports`'s), and `hasExpansions`, whether an inline or macro expansion was made in the file.
  `unusedImports` throws where the analysis is unavailable (the program did not type), so a run that cannot answer
  fails with exit 2 rather than finding nothing. `symbol` and `info` (SemanticDB's symbols and signatures, which
  the next part ports from dotty's `SemanticSymbolBuilder` and `TypeOps`), `tpe`, `<:<` and `references` are not
  there yet.
- `Tree`: every node of the parser's trees, named and laid out as scalameta's: `Source`, `Pkg`, `Import`,
  `Importer`, `Importee.Name`/`Rename`/`Unimport`/`Wildcard`/`Given`/`GivenAll`, `Defn.Val`/`Var`/`Def`/`Class`/
  `Trait`/`Object`/`Enum`/`EnumCase`/`Given`/`GivenAlias`/`Type`, `Decl.*`, `Template`, `Init`, `Term.Param` and
  `Term.ParamClause` (`isImplicit`, `isUsing`), `Type.Param`, `Mod.*` (each modifier where it is written, and
  `Mod.Annot`), `Term.*` (`Name`, `Select`, `Apply`, `ApplyInfix`, `ApplyUnary`, `ApplyType`, `Function`, `Match`,
  `Block`, `If`, `While`, `For`, `Return`, `Throw`, `New`, `Interpolate`, `AnonymousFunction` and `Placeholder` of
  `_ + 1`, ...), `Lit.*`, `Case`, `Enumerator.*`,
  `Pat.*`, `Type.*`; a kind with no class of its own is an `Other`. Each has `pos`, `syntax` (its text), `parent`,
  `children`, `tokens`, `collect` and `traverse` (pre-order), and the named children of its kind (`fun`, `args`,
  `tparams`, a higher-kinded `Type.Param`'s too (`A` of `F[+A]`, its variance a `Mod.Covariant`), and a type
  lambda's, a polymorphic function type's and literal's (`A` of `[A] =>> List[A]`, a polymorphic function's
  binder's context bounds among its children), `Type.Annotate`'s `tpe` and `annots` (`@ann(1 == 2)` a `Mod.Annot`),
  `qual`, `name`, `mods`, `pats`, `rhs`, `templ`, ...), with the extractors scalameta's rules match on
  (`Term.Apply(fun, args)`, `Term.Apply.Initial(...)`, `Term.Select(qual, name)`, `Term.Name(value)`, `Lit.Unit()`).
  An expression in parentheses of its own is a `Term.Parens`, which scalameta drops. A `Template` is scalameta's:
  from the token after the header (past `extends`) to its definition's end, its braces and an empty body in it; a
  `Term.ParamClause` spans its parentheses, an empty `()` too.
  What the parser makes up (a case class's companion, a `derives` clause's given, a pattern definition's vals, the
  lambda a higher-kinded parameter's bound is made over its clause, whose written bound stands in its place, a
  polymorphic function's evidences) is no node: a pattern definition
  is the `Defn.Val` written. Two handles of one node are equal; a node of a document the rules no longer read throws
  when asked anything.
- `Token`: every byte of the file is in one, trivia included: `kind` is scalameta's token class (`KwVar`, `Ident`,
  `Semicolon`, `Comma`, `LeftBrace`, `Space`, `Tab`, `LF`, `CRLF`, `Comment`, `Constant.Int`,
  `Interpolation.Part`, ...), `text`, `value` (a comment's text inside its delimiters, an identifier's name without
  its backquotes), `pos`, `isKeyword`,
  `isTrivia`. An end marker is its `end`, an `Ident`, and its name, an `Ident` or, where the name is a keyword, that
keyword's token (`KwWhile` of `end while`), as scalameta's tokenizer scans it; so is a quoted identifier, `'x` in a
splice, its quote a `Quote` (scalameta's `MacroQuote`) and its name the name's own token. Where scalameta's stream
differs otherwise, no shipped rule reads the difference: an interpolation's opening quote and its `$` and `${` are
`Interpolation.Delimiter` (scalameta's `Start`, `SpliceStart`), and teq makes none of scalameta's empty tokens
(`SpliceEnd`, an empty `Part`, `BOF`, `EOF`); a run of spaces is one `Space`; `#` and a splice's `$` are an `Ident` and a
`Splice`; a script's header (`#!` or `::#!` through a line starting `!#`, scalac's) is one `Shebang`, where scalameta's
is the first line alone.
- `Position`: `start` and `end` in UTF-16 units of `input.text` (a `String`'s, as scalameta's), `startLine`,
  `startColumn`, `endLine`, `endColumn` from 0, `text`; `Position.Range(input, start, end)` from offsets of the text,
  a regex match's among them. The lints print in the compiler's columns.
- `Patch`: `addLeft` and `addRight` of a token or a tree, `removeToken`, `removeTokens`, `replaceToken`,
  `replaceTree` (the tree's tokens removed and the text added right of the first, scalafix's), `removeImportee`
  (dotty's `CheckUnused.checkImports` edits: the importer goes with its last importee, the import with its last
  importer, a clause with one importee kept written as that importee's import), `addGlobalImport("a.b.c")`
  (scalafix's `ImportPatchOps`: a line `import a.b.c` right of the last token of the file's leading imports, else of
  `fallbackToken`'s token: in a package clause, before its first statement's line and that statement's leading
  comment, inside its braces where it has them; in a file whose first statement is no package clause, at its start;
  unless the file has it written in a top-level import), `lint(Diagnostic(...))`, `+`, `empty`, `atomic`, and
  `asPatch` of a collection or an option of patches. A token, an importee or a position of another document than
  the one the rule reads fails the rule, in a patch as in an accessor (a token keeps its `kind` once read).
- `Diagnostic(id, message, position, explanation = "", severity = LintSeverity.Error)`, or a class implementing
  `message`, `position`, `severity`, `categoryID`. Its full id is `<rule>.<id>`.
- `Configuration` and `Conf`: `c.conf.section(name)` (an empty one where the configuration has none), `get`,
  `keys`, `items`, `string(key, default)`, `boolean`, `int`, `strings`, `asRegex`, `place` (the setting's file,
  line and column for a message); `Configured.Ok`/`NotOk` with `map`, `flatMap` and `Configured.all`.

## Patches, suppressions and rounds

- Each rule's edits merge as scalafix's: on one token, additions concatenated in the order made, a removal and an
  addition a replacement. Across rules, edits that are the same count once and additions to a token none removes are
  joined in the configured order of the rules; a removal by one rule and an addition by another on one token is a
  conflict, exit 2 with both rules named. The edits are applied in dotty's `Rewrites.Patches.apply` order: each
  distinct edit once, sorted by its start alone (a stable sort over the tokens' edits, then the import removals', then
  the added imports), one that overlaps the next refused (exit 2, both rules named). The whole run is planned before a
  file is written; a file two configurations share must be edited alike by both, a failure in either refuses it, and
  the lints of both are printed, each once. A file is written beside itself, then renamed over the original.
- Untouched bytes stay as they are. A `\n` a rule inserts is written as the file's line ending, that of its first
  line break. An edit that would end between a CR and its LF ends after the LF, the line's CRLF written after its
  text.
- Suppressions are scalafix's `EscapeHatch`: `// scalafix:off [rules]` to `// scalafix:on [rules]`,
  `// scalafix:ok [rules]` after the tree it ends or on its line, `@SuppressWarnings(Array("[scalafix:]rule"))` or
  `"all"` on a definition. A suppressed lint is dropped; an `atomic` patch with an edit where its rule is suppressed
  is dropped whole; other edits are not suppressed, as scalafix does not suppress them. A lint made inside an
  atomic patch goes with the patch: dropped where its edits are suppressed, reported where they are not. An anchor,
  or an annotated rule written `scalafix:`, that suppresses nothing is the warning `UnusedScalafixSuppression`.
- At most two rounds. The rules run over the files as read; where they edit, every program that reads an edited
  file is typed again over what they left (a dependent's, whose own files are as they were, too; one that no longer
  types refuses the whole run before a file is written) and the rules run once more, where only `NoUnusedImports`'
  removals may edit (an import a rewrite made unused, as `Nil` rewritten `List.empty` leaves `import scala.Nil`);
  any other edit then is exit 2, the rule rewriting its own output. In the second round a file the first left as it
  was reads alike, so only the semantic rules run over it again (another file's edits may leave its imports
  unused), the others' results being the first round's. A run that edits nothing types once.
- Each program is typed as `teq compiler check` types it, over compile's workers: `--threads` (or `TEQ_THREADS`), else
  compile's count for that round's text (one under 512 KiB or where one unit holds over half of it, else the cores up
  to eight); the unused imports and the expansions are the merged typer's, as one worker's. A parallel attempt that
  gives way (a macro's run that changed state other runs share, two diagnostics at one place) is the round's: its
  program is typed again by one worker from the round's texts, nothing of the attempt kept and the rules not configured
  again, so that the output is the one-worker run's; the record and the trace say so, nothing else does. A typing waits
  for every rule worker started to be ready, and the rules for the typing's end. Where the system gives one of the
  typing's threads no thread (the address space bounded: a body phase's or an inlining phase's worker, a merge thread),
  the attempt is given up as above and the round typed by one worker, the record naming the thread; a rule worker it
  gives no thread leaves the rules on the workers they have (the record's `worker-refused` line), and the first rule
  worker's or the typing thread's refusal is exit 2 with the reason. The run's allocator is a resident
  process's (`TEQ_RESIDENT=0` the batch one, for measuring): each typing's threads hand back what they hold as they end
  and the next typing takes it, so that a round's typing does not strand the last one's memory.
- The rules run over a round's documents on workers of their own, each with the rules' program and an interpreter of
  its own, when every configured rule is one teq ships (none keeps state across documents): as many as `--threads`
  (or `TEQ_THREADS`; the cores by default) and the round's documents allow, started once its typing has ended. A
  `file:` rule keeps one worker for the whole run, so that its instance keeps its state across documents, jobs and
  rounds. The documents' results are planned in the documents' order, whichever worker ends first, so the output is
  the one-worker run's byte for byte.

## The rules teq ships

Each is ported from its repository (tests/fix/REFERENCES.md: the revisions, the licences, the fixtures run as
tests/fix's cases), its file keeping the repository's notice.

- `DisableSyntax` (scalafix): every setting of its `DisableSyntaxConfig` (`keywords`, `noVars`, `noThrows`,
  `noNulls`, `noReturns`, `noWhileLoops`, `noAsInstanceOf`, `noIsInstanceOf`, `noSemicolons`, `noTabs`, `noXml`,
  `noCovariantTypes`, `noContravariantTypes`, `noDefaultArgs`, `noValInAbstract`, `noImplicitObject`,
  `noImplicitConversion`, `noFinalVal`, `noFinalize`, `noValPatterns`, `noUniversalEquality`,
  `noUniversalEqualityMessage`, `regex` as strings or objects with `id`, `pattern` or `regex = { pattern,
  captureGroup }`, `message` and its `{$n}` substitution), its section also `disableSyntax`. `noXml` is read and
  reports nothing: teq reads no XML literal. A `finalize` of no parameters (or one empty clause) is reported whatever
  `noFinalize` says, where no other check claims the definition, as scalafix's `FinalizeMatcher` reports it after its
  other cases (`noDefaultArgs` claims every definition).
- `CatsImplicitsImport` (xuwei-k/scalafix-rules): a warning on `import cats.implicits._`; the setting `message`.
- `TypelevelAs` (typelevel-scalafix): the lints of `.map(_ => <literal>)` and `.as(())`.
- `EmptyCollectionsUnified` (scalafix-unified): `List()`, `List[T]()`, `Set()`, `Map()` and `Nil` written as
  `.empty` (and `List()` in a pattern as `Nil`), each atomically, its quasiquotes written as the patterns they
  stand for.
- `NoUnusedImports` (teq's): every importee the typer's unused-import analysis reports removed through dotty's edits; a
  hiding selector (`a as _`) goes in the same run as its clause's last wildcard, as `CheckUnused.matchingSelector`
  counts it used only beside one. In a file where an expansion was made (an inline method's or a macro's), scalac
  credits imports through the expansion that teq's analysis does not yet see (CheckUnused's `resolveUsage` over the
  expansion's trees): there each unused importee is the warning `NoUnusedImports.unverified` and stays.

## Debugging a rule

`TEQ_FIX_TREE=1` prints each document's tree, a node a line with its text; `TEQ_FIX_TRACE=1` the counts of each
document's nodes, tokens, edits and lints and each rule's failure, then the run's phases (the rules' program, the
typing, the rules — the rounds' wall time, the workers, their documents' time summed, its CPU time and the longest document's —, the
planning) in milliseconds and what the allocator reserved; and a line for each typing attempt (its workers asked and
typed with, its outcome and, where it gave way, why; its parts: reading and parsing, the jars, the typing as `teq
compiler check --time` counts its `type` phase, with `run`'s phases, the documents, the jar caches, the attempt's exit)
and for each round the wait for what its merges gave back to be dropped.
`TEQ_FIX_RECORD=<file>` writes the run's execution record, a line an event: `run` (whether the rules are the shipped
ones alone, the workers, and whether a semantic rule is configured, so that the typings read the unused imports), each worker's `worker-start` and `worker` lines (when it was started and when it was
ready), each typing attempt's `typing` line (its job, round and number, the workers asked and those `Typer::run` typed
with, whether the system refused the overlays' mappings, so that one worker typed it in place, or one of the typing's
threads (a worker of either phase, a merge thread), so that the attempt was given up and one worker typed the round
again, whether it forked, its outcome, `one`,
`joined`, `retried` or `refused`, when it began and ended, why it gave way), each round's `round` (its job, its number,
its documents and its first generation), a `dispatch` and a `done` line a document (its slot, generation and worker, and
when its rules began and ended, in microseconds since the run began, and the worker's CPU time between), a `serial` line where a round runs on one worker,
and `round-end` (the round's wall time and the sums of its documents' times and CPU times): the fix suite reads it to tell a parallel
run from a serial one. `TEQ_FIX_INPUTS=<file>` writes each job's inputs as the compiler resolves them (its roots, the links it follows, its
exclusions), a line `<job>\t<path>` each, and does nothing else: the corpus a measurement of bench/fix runs over.
`TEQ_FIX_SCHEDULE` (`reverse`, `shuffle:<seed>`), `TEQ_FIX_BARRIER=<workers>` and
`TEQ_FIX_PLANT` (`serial`, `completion`, `serial-typing`) are that suite's switches: the order the documents are handed
out in, a wait until that many workers have begun, and the mutation controls its checks must refuse (`serial-typing`
types every program by one worker, the rules' pool left as asked). So are `TEQ_FIX_TYPING_THREADS=<n>` and
`TEQ_FIX_RULE_THREADS=<n>`, the typing's and the rules' counts apart; `TEQ_FIX_TYPING_WORK=1`, which turns the typer's
counters on for each typing and writes a `typing-item` line for each item of its queue (the worker, the item's index in
its file, when it began and ended, its file), the witness that several workers typed at once; and
`TEQ_FIX_BOOT_DELAY=<ms>`, which holds every rule worker but the first that long before its bootstrap.
`TEQ_FIX_INDEX=off` is bench/fix's, for its comparison with `teq compiler check`, which indexes nothing: the typer makes
no index, and `hasExpansions` is false everywhere.
