import teq.fix.*

/** Reports the symbol of each type's name and of each member selected from a simple name
  * (tests/fix/Driver/JavaMemberClassSymbols). */
final class ClassSymbols extends SemanticRule("ClassSymbols"):
  override def fix(using doc: SemanticDocument): Patch =
    doc.tree.collect {
      case name: Type.Name if !name.symbol.isNone => at(name)
      case Term.Select(_: Term.Name, name) if !name.symbol.isNone => at(name)
    }.asPatch

  private def at(name: Name)(using SemanticDocument): Patch =
    Patch.lint(Diagnostic("at", name.symbol.value, name.pos, severity = LintSeverity.Warning))
