# teq fix: lint and rewrite rules

`teq fix` (docs/CLI.md) runs scalafix-shaped rules over a build's sources with no sbt and no JVM. A rule is Scala,
run by teq's interpreter against the API of the std's package `teq.fix`, shaped after scalafix's `scalafix.v1` so
that a scalafix rule reads alike and ports with local changes; it reads the tree scalameta's parser makes of each
file (teq's port of it, over the tokens the compiler's lexer kept) and the types of the files the compiler typed in
the same process, and returns patches and lints, which `teq fix` plans, checks and applies. A program teq does not
type is exit 2 before any rule runs, the syntactic rules among them, and so is a file scalameta's parser refuses, as
scalafix refuses it: its message at its place.

## Configuration

`.scalafix.conf` beside `teq.lock` (or the file `--config` names), read as scalafix reads it, typesafe-config 1.4.6's
HOCON under metaconfig 0.18.6, its includes read where they stand and its substitutions resolved once the whole is
read. `key = value`, `key: value`, `key { ... }` and `key += value`, dotted keys (`DisableSyntax.noVars = true`),
objects merged as HOCON merges them, lists with or without commas and over lines, quoted and bare strings (a quoted
one's characters as written, its spaces among them), the values written on one line concatenated with what stands
between them (`"print""ln"`, `${a} [b]`), numbers, booleans, `null`, `#` and `//` comments; `include "name"` (beside
the including file, a name without an extension its `.conf` over its `.json`), `include file("name")` (from the
working directory) and `include required(...)`; `${path}` and `${?path}`, looked up in the whole configuration (a
reference in a file included inside an object under that object's path first), then as the environment's variable of
that name. So `DisableSyntax.regex = ${DisableSyntax.regex} [...]` after an `include` extends the included list, as
scalafix extends it. Each value keeps its file, line and column, which its errors name. Where teq parts from
typesafe-config: the fields resolve in the order written, not in its hash map's, which changes only which of several
errors comes first and how a cycle of substitutions is named; `url(...)`, `classpath(...)`, an included file that is
not beside the including one (which typesafe-config would look for on the JVM's class path) and an included Java
properties file are errors at their place; what scalafix crashes on (an incompatible concatenation, whose error is at
its left value's place as typesafe-config gives it, the referenced one's for a substitution; a required include that
is missing; an empty path) is an error at its place; an object with numeric keys beside a list is not made a list, and
`${X[]}` reads no list of variables. A triple-quoted string and a character typesafe-config reserves outside quotes
(``$=:+^?!@*&\`` and the backquote) are errors at their place too: exit 2, never skipped.

A lock sbt-teq exports carries what sbt-scalafix does in each project's `compile` and `test` configurations, its
`scalafix` record (docs/TOOLING.md, "The lockfile and the launchers"): the configuration file sbt-scalafix resolves
there (`config`, from `scalafixConfig`) and whether it skips the configuration (`skip`, from `scalafix / skip`, or a
project without the plugin). `teq fix` runs each configuration under its file's rules, the rest under the root's
`.scalafix.conf`, all as one run (below, "The whole run is planned"), and leaves a skipped configuration out unless
`project/configuration` names it; `--config` is the file of every configuration. A lock without the records is every
configuration under the root's file; one whose configurations are all skipped runs nothing, its execution record
(`TEQ_FIX_RECORD`) written empty.

```
rules = [DisableSyntax, NoUnusedImports, "file:fix/NoToString.scala"]
DisableSyntax.noVars = true
DisableSyntax.regex = [{ id = offensive, pattern = "[Pp]imp", message = "Please consider a less offensive word" }]
lint.error.includes = ".*"
```

- `rules` (or `rule`): a rule's name, or `file:<path>`, a Scala file or a directory of them relative to the
  configuration's directory, every concrete class there extending `teq.fix.Rule` with a constructor of no arguments a
  rule under its `name`. `--rules A,B` replaces the list, its names chosen among the rules the configuration
  discovers: the shipped ones and every rule of its `file:` entries and of a `file:` entry `--rules` names. A name of
  no rule is an error naming it. A `class:<name>` entry is not read: it names no rule (exit 2).
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
  `fix(using doc: SemanticDocument): Patch`; `withConfiguration` returns the configured rule or `Configured.NotOk`;
  `description`, `isLinter`, `isRewrite` and `isExperimental` are scalafix's metadata, which the verb does not read.
- `SyntacticDocument`: `input` (its `text` and `path`), `tree` (the `Source`), `tokens`.
  `SemanticDocument` adds the typer's answers: `unusedImports`, the importees the typer's unused-import analysis
  reports (`-Wunused:imports`'s), and `hasExpansions`, whether an inline or macro expansion was made in the file.
  `unusedImports` throws where the analysis is unavailable (the program did not type), so a run that cannot answer
  fails with exit 2 rather than finding nothing. `symbol` and `info` (SemanticDB's symbols and signatures, ported
  from dotty's `SemanticSymbolBuilder` and `TypeOps`) are being built: where a semantic rule is configured, the round's
  typing thread serves the rules' SemanticDB questions after its typing until the round's last document is done; at
  the round's first question it types the round's texts (the program's files as the round's typing read them, never
  the disk again) by one worker keeping the forms dotty's types keep, and answers from that typing (src/fix/service.rs), a question it cannot answer the asking rule's failure; it answers the
  document's header so far. `tpe`, `<:<` and `references` are not there.
- `Tree`: every node of scalameta 4.17.4's parser's tree of the file (teq's port of `ScalametaParser`, src/fix/parse;
  a script's own text under the dialect with top-level terms, as scalafix's `dialectForFile`), named and laid out as
  scalameta's: `Source`, `Pkg`, `Import`,
  `Importer`, `Importee.Name`/`Rename`/`Unimport`/`Wildcard`/`Given`/`GivenAll`, `Defn.Val`/`Var`/`Def`/`Class`/
  `Trait`/`Object`/`Enum`/`EnumCase`/`Given`/`GivenAlias`/`Type`, `Decl.*`, `Template`, `Init`, `Term.Param` and
  `Term.ParamClause` (its `mod` the clause's `implicit` or `using`), `Type.Param`, `Mod.*` (each modifier where it is written, and
  `Mod.Annot`), `Term.*` (`Name`, `Select`, `Apply`, `ApplyInfix`, `ApplyUnary`, `ApplyType`, `Function`, `Match`,
  `Block`, `If`, `While`, `For`, `Return`, `Throw`, `New`, `Interpolate`, `AnonymousFunction` and `Placeholder` of
  `_ + 1`, ...), `Lit.*`, `Case`, `Enumerator.*`,
  `Pat.*`, `Type.*`. Each has `pos`, `syntax` (its text), `parent`,
  `children`, `tokens`, `collect` and `traverse` (pre-order), and the named children of its kind (`fun`, `args`,
  `tparams`, a higher-kinded `Type.Param`'s too (`A` of `F[+A]`, its variance a `Mod.Covariant`), and a type
  lambda's, a polymorphic function type's and literal's (`A` of `[A] =>> List[A]`, a polymorphic function's
  binder's context bounds among its children), `Type.Annotate`'s `tpe` and `annots` (`@ann(1 == 2)` a `Mod.Annot`),
  `qual`, `name`, `mods`, `pats`, `rhs`, `templ`, ...), with the extractors scalameta's rules match on, shaped as
  scalameta 4.17's: the bare ones and `Initial` give a clause's values as a list (`Term.Apply(fun, args)`,
  `Term.Apply.Initial(...)`), `After_4_6_0` the clause (`Term.Apply.After_4_6_0(fun, argClause)`), and
  `Term.Select(qual, name)`, `Term.Name(value)`, `Lit.Unit()`. An application's arguments are its `Term.ArgClause`,
  from the token after its function to its end (a `using` one with its `Mod.Using` after the values); an infix
  operation's right operand is its clause, beside an empty `Type.ArgClause`; an `Init`'s clauses are its `argClauses`;
  a type application's arguments its `Type.ArgClause`; `A | B` and `A & B` are `Type.ApplyInfix`; an extractor's
  patterns its `Pat.ArgClause`; `xs*` is a `Term.Repeated` and `e: @a` a `Term.Annotate`. A def's type parameters and
  clauses are its `Member.ParamClauseGroup` (`paramClauseGroups`; `tparams` and `paramss` its first group's, as
  scalameta's); a class's, trait's, enum's and enum case's its `Type.ParamClause` and `Ctor.Primary` (with a
  constructor's access modifier), each empty and zero-width where scalameta's parser stands where none is written, as
  a type parameter's `Type.Bounds` and `Type.ParamClause` are; a clause's `implicit` or `using` is its `mod`, after its
  parameters, and each parameter's modifier too, `val` and `var` a parameter's `Mod.ValParam` and `Mod.VarParam`;
  `def this` is a `Ctor.Secondary` with its `Ctor.Block`; an enum case starts at its `case`, `case A, B` one
  `Defn.RepeatedEnumCase`; an extension's methods are a `Defn.ExtensionGroup`'s body (one method, or a `Term.Block` of
  them); a function type's parameters its `Type.FuncParamClause`, a named one a `Type.TypedParam`; a refinement written
  alone a `Type.Refine` of no `tpe`, and one of a parent from that parent's own start, inside the parentheses of a
  parent written in them (scalameta's `atPos(tpe, ..)`); a named tuple type's elements `Type.TypedParam`s; `?` a
  `Type.Wildcard` over its `Type.Bounds`.
  There is no node of parentheses, as scalameta has none: a parenthesized expression or type is the expression or the
  type, its parentheses its parent's tokens, which the parent spans. A parenthesized pattern spans its parentheses
  (scalameta's `simplePattern`), but an infix pattern's operand after its operator, whose parentheses are its
  `Pat.ArgClause`'s (`h :: (t)`) or the start of the infix it is the left operand of (`(t) :: Nil` after `::`), the
  operand keeping the parentheses inside them; a tuple there is the clause (`a ~~ (b, c)`). Every extent is the
  parser's (`atPosWithBody`). A written block `{ e }` is a `Term.Block`, a colon argument one from its colon and a splice
  `${e}` one from its `{`; a lambda's parameters are its `Term.ParamClause`, `x ?=> e` a `Term.ContextFunction` and
  `{ case ... }` a `Term.PartialFunction`; a match's and a `catch`'s cases a `Term.CasesBlock` (its braces, else from its
  first case, a `;` after a case its own), `x.match` a `Term.SelectMatch`, `try e catch h` a `Term.TryWithHandler`; a
  for's enumerators a `Term.EnumeratorsBlock` (its delimiters, else from the first), a guard from its `if` and a case
  generator from its `case`; `xs size` a `Term.SelectPostfix`; `inline if` from its `inline`, a named tuple's elements
  `Term.Assign`s; an end marker a `Term.EndMarker` beside what it closes (with a definition's body in a `Term.Block`).
  A pattern's `a | b | c` is right-nested, `rest*` a `Pat.Repeated`, `given T` a `Pat.Given`, a type variable a
  `Type.Var` over its name; a match type's cases a `Type.CasesBlock`, its variables `Type.Name`s and `_` a
  `Type.PatWildcard`; a refinement's declarations a `Stat.Block`; a template's `derives` types follow its body. A
  package clause's statements are
  its `Pkg.Body`: its braces or its colon's region as written, else from its first statement; a package block is a
  statement of the clause around it. A `Template` is scalameta's: from its `extends` where written, else from its body
  (a given's and an anonymous class's from their first parent), to its definition's end; its `Template.Body` the
  braces or the colon's region as written (a given's `with` body from its first statement), with its `Self` (`self: T
  =>`), else empty at the definition's end. An importee's name is a `Name.Indeterminate`. A `Term.ParamClause` spans
  its parentheses, an empty `()` too.
  What the parser makes up (a case class's companion, a `derives` clause's given, a pattern definition's vals, the
  lambda a higher-kinded parameter's bound is made over its clause, whose written bound stands in its place, a
  polymorphic function's evidences, an implicit class's conversion) is no node: a pattern definition is the
  `Defn.Val` written.
  The node contract is scalameta 4.17.4's `Trees.scala` whole: every class teq makes is scalameta's, with every field
  of it under the field's name and type, and every value, syntax and derived accessor as scalameta computes them (the
  accessors std/fix/tree.scala has are scalameta's, its legacy ones among them: `Defn.Var.rhs` beside `body`,
  `Template.self` the empty `Self(Name.Anonymous(), None)` where none is written, `paramss` of a def's first group; no
  accessor scalameta lacks). A literal's value is scalameta's: an `Int`'s or a `Long`'s number, a `Lit.Double`'s or a
  `Lit.Float`'s `format` the `java.math.BigDecimal` scalameta's `AnyDecimal` makes of the digits (trailing zeros
  stripped where the scale is over 1 or an exponent moved the point, `1e3` `1E+3`, the sign apart, a zero unsigned)
  and its `value` that number's; a unary operator before a literal folded into it as scalameta's parser folds it
  (`-1`, `!true`), else a `Lit.WithUnary` (`-'a'`, its value `("-", 'a')`); a name's value is its token's, no
  backquotes, a comment around it no part of it; an interpolation's parts are its `Lit.String`s, `$$` and `$"` the
  character after the `$` and, in an `s` or `f` interpolation, a unicode escape its character; an `if` without `else` has an empty `Lit.Unit` at its `thenp`'s end, a bare `return`
  one after it; `var x: T = _` has its `Term.Placeholder`; `Name.Anonymous` stands where scalameta's parser makes one
  (a primary constructor's, an `Init`'s and an anonymous given's name, an unqualified access modifier's `within`, the
  qualifiers of `this` and `super`, an anonymous parameter's name); `private[p]`'s `within` is a `Name.Indeterminate`,
  `private[this]`'s a `Term.This`; `new C(1)(2)` is one `Term.New` whose `Init` has both clauses; a single extension
  method is its group's body, whatever its line; `A with B` is a `Type.With`; `C as c` of a context bound a
  `Type.BoundsAlias`; a quoted pattern a `Pat.Macro`; a kind projector's type a `Type.AnonymousLambda` over its `*`s'
  `Type.AnonymousParam`s; `Template.earlyClause` and `Type.Bounds.view` (Scala 2's) are empty. The `Nodes` cases of
  tests/fix.scala hold it: a fixture per class, teq's structure of it (support/DumpStructure.scala: every node's
  fields, scalar values and syntax, and each derived accessor's answer) against scalameta's (`structure.txt`,
  support/MetaStructure.scala, the accessors called on scalameta's classes). teq makes none of these classes, which no source it
  compiles yields:
    1. scalameta's private or deprecated ones: `Term.ApplyUsing`, `Type.And`, `Type.Or`, `Type.Method`,
       `Type.ImplicitFunction`, `Mod.Super`;
    2. those scalameta's parser makes of no source as teq's rules parse it: `Lit.Byte`, `Lit.Short`, `Lit.IntXL`,
       `Lit.FloatXL`, `Tree.Comment` and `Tree.Comments` (comments captured), `MultiSource`;
    3. forms scalac 3.8.4 rejects: `Term.Do`, `Type.Existential`, `Lit.Symbol`, `Defn.Macro`, `Decl.GivenAnonymous`,
       `Type.Macro`, `Type.AnonymousName` (a declared type of an annotation alone, `val x: @a = 1`);
    4. experimental features teq does not compile: `Mod.Erased`, `Type.FunctionArg`, `Mod.Tracked`, `Mod.Into`,
       `Type.Assign`, `Term.SubMatch`, capture checking's `Term.CapSetName`, `Type.CapSetName`, `Type.CapturesAny`,
       `Type.CapturesSet`, `Type.Capturing`, `Type.PureFunction`, `Type.PureContextFunction`, `Type.PureByName`;
    5. forms teq's parser does not read: XML literals (`Term.Xml`, `Pat.Xml`), a quoted type pattern's definitions
       (`Type.Block`, pinned as tests/pending-cases/quoted_type_block.scala).
  Two handles of one node are equal; a node of a document the rules no longer read throws when asked anything.
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
  (token removals, as scalafix's `ImportPatchOps` converts every rule's removed importees: the importee's tokens and
  the comma before it, or after it where it leads the removed ones; the importer with its last importee, the import
  with its last importer and the line break and spaces before it; the braces of a clause left with one name or
  wildcard), `addGlobalImport("a.b.c")`
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
  joined in the configured order of the rules; a token one rule removes and another adds to is the addition, as
  `PatchInternals.merge` makes it, both additions where two rules add the same text on the same side, unless two
  rules add different text to it, a conflict, exit 2 with both rules named. The import removals' token removals merge with the rules' edits alike. Where scalafix's own
  output would not parse, teq edits so that it does: an importee written after a comment that follows the comma
  before it, which scalafix removes without a comma (its `removeUpToFirstComma` stops at the comment, the comma left
  alone on its line), goes with its comma as dotty's `checkImports` edit removes it (the importee with what follows
  it to the next, the last from the comma after the importee kept before it). A trailing comma the braces keep a line
  before their close parses, as Scala's, and scalafix's edit stands there. An import removed whole also loses
  what dotty's `editPosAt` deletes with it: its indentation, the blanks after it, and where its line is otherwise
  blank the line, with the blank line before it where one stands on each side; its line loses one line break of the
  two around it, the one another rule's edit removes where exactly one of them is, else the one before it
  (scalafix's). The edits are applied in dotty's `Rewrites.Patches.apply` order: each distinct edit once, sorted by
  its start alone (a stable sort over the tokens' edits, then the added imports), one that overlaps the next refused
  (exit 2, both rules named). The whole run is planned before a file is written, the configurations of every
  configuration file over the files as read, a rule one of the files names that teq lacks refusing it first; a file two
  configurations share must be edited alike by both, whatever their files, a failure in either refuses it, and the
  lints of both are printed, each once. A file is
  written beside itself, then renamed over the original.
- Untouched bytes stay as they are. A `\n` a rule inserts is written as the file's line ending, that of its first
  line break.
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
- `NoUnusedImports` (teq's): every importee the typer's unused-import analysis reports removed (`removeImportee`); a
  hiding selector (`a as _`) goes in the same run as its clause's last wildcard, as `CheckUnused.matchingSelector`
  counts it used only beside one. In a file where an expansion was made (an inline method's or a macro's), scalac
  credits imports through the expansion that teq's analysis does not yet see (CheckUnused's `resolveUsage` over the
  expansion's trees): there each unused importee is the warning `NoUnusedImports.unverified` and stays.

## Debugging a rule

`TEQ_FIX_TREE=1` prints each document's tree, a node a line with its text; `TEQ_FIX_TRACE=1` the counts of each
document's nodes, tokens, edits and lints and each rule's failure, then the run's phases (the rules' program, the
typing, the rules — the rounds' wall time, the workers, their documents' time summed, its CPU time and the longest document's —, the
planning, the SemanticDB answers — each round's service from its first question to its last answer, its typing among
them) in milliseconds and what the allocator reserved; and a line for each typing attempt (its workers asked and
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
when its rules began and ended, in microseconds since the run began, the worker's CPU time between, and the SemanticDB
questions its rules asked), a `serial` line where a round runs on one worker, a `serve` line where a semantic rule is
configured (the round's SemanticDB service: the questions it answered, whether it typed, when it began typing and ended),
and `round-end` (the round's wall time and the sums of its documents' times and CPU times): the fix suite reads it to tell a parallel
run from a serial one. A `doc` line names each document of each job's round, with the job's key, the configuration
file and the number of lints that job gave it, before the jobs' lints of one file are merged into the ones printed,
each lint a `lint` line (its line, column and byte offsets in the round's text, severity, id, and message as a JSON
string): a comparison with scalafix, which prints a lint once per task that gave it, counts them there. `TEQ_FIX_INPUTS=<file>` writes each job's inputs as the compiler resolves them (its roots, the links it follows, its
exclusions), a line `<job>\t<path>` each, and does nothing else: the corpus a measurement of bench/fix runs over.
`TEQ_FIX_SCHEDULE` (`reverse`, `shuffle:<seed>`), `TEQ_FIX_BARRIER=<workers>` and
`TEQ_FIX_PLANT` (`serial`, `completion`, `serial-typing`, `query-failure`) are that suite's switches: the order the
documents are handed out in, a wait until that many workers have begun, and the mutation controls its checks must refuse
(`serial-typing` types every program by one worker, the rules' pool left as asked; `query-failure` fails every answer
of the SemanticDB service, as an extraction's error would). So are `TEQ_FIX_TYPING_THREADS=<n>` and
`TEQ_FIX_RULE_THREADS=<n>`, the typing's and the rules' counts apart; `TEQ_FIX_TYPING_WORK=1`, which turns the typer's
counters on for each typing and writes a `typing-item` line for each item of its queue (the worker, the item's index in
its file, when it began and ended, its file), the witness that several workers typed at once; and
`TEQ_FIX_BOOT_DELAY=<ms>`, which holds every rule worker but the first that long before its bootstrap.
`TEQ_FIX_INDEX=off` is bench/fix's, for its comparison with `teq compiler check`, which indexes nothing: the typer makes
no index, and `hasExpansions` is false everywhere.
`TEQ_FIX_SEMANTICDB=<dir>`, a test's, types the run as for a semantic rule, by one worker keeping the retained forms
(`--semantic-types`), and writes each file's SemanticDB document of round one under `<dir>/META-INF/semanticdb/`, as scalac's `-Xsemanticdb -semanticdb-target <dir>` with its
`-sourceroot`, `TEQ_FIX_SEMANTICDB_ROOT` (by default the working directory), would; tests/semanticdb compares it with
scalac 3.8.4's. Beside each document, `<file>.forms` lists the types of the file's members with the retained forms the
typer keeps (`Phantom[String]`, `Int @Ann`), a view of the store the same suite checks against scalac's typed tree.
