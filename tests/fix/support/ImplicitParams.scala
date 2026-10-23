import teq.fix.*

/** Reports each parameter clause whose `mod` is `implicit` with its syntax and its parameters' syntax and modifiers
  * (tests/fix/Driver/ImplicitLambdaParams). */
final class ImplicitParams extends SyntacticRule("ImplicitParams"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect {
      case c: Term.ParamClause if c.mod.exists(_.isInstanceOf[Mod.Implicit]) =>
        val params = c.values.map(p => s"`${p.syntax}` mods ${p.mods.map(_.syntax).mkString(",")}").mkString("; ")
        Patch.lint(Diagnostic("clause", s"`${c.syntax}` mod ${c.mod.map(_.syntax).getOrElse("none")}: $params", c.pos, severity = LintSeverity.Warning))
    }.asPatch
