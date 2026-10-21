import teq.fix.*

/** Reports each type lambda with its text: the written ones alone, none the parser makes up for a higher-kinded
  * parameter's bound (tests/fix/Driver/MadeLambdas). */
final class Lambdas extends SyntacticRule("Lambdas"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case t: Type.Lambda => Patch.lint(Diagnostic("lambda", t.syntax, t.pos, severity = LintSeverity.Warning)) }.asPatch
