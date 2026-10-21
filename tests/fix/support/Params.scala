import teq.fix.*

/** Reports each type parameter with its text: a type lambda's and a polymorphic function's binders among them
  * (tests/fix/Driver/LambdaBinders, `LambdaBindersI13541`). */
final class Params extends SyntacticRule("Params"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case p: Type.Param => Patch.lint(Diagnostic("param", p.syntax, p.pos, severity = LintSeverity.Warning)) }.asPatch
