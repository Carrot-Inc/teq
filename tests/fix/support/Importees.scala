import teq.fix.*

/** The importees' extractors as scalameta's (`Importee.Given(tpe)`, `GivenAll()`, `Wildcard()`), a lint each naming
  * what it matched (tests/fix/Driver/ImporteeExtractors). */
final class Importees extends SyntacticRule("Importees"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect {
      case i @ Importee.Given(tpe) => Patch.lint(Diagnostic("given", "given " + tpe.syntax, i.pos))
      case i @ Importee.GivenAll() => Patch.lint(Diagnostic("givenAll", "given", i.pos))
      case i @ Importee.Wildcard() => Patch.lint(Diagnostic("wildcard", "*", i.pos))
    }.asPatch
