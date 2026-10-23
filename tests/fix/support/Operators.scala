import teq.fix.*

/** Reports each infix type's operator (tests/fix/Driver/WithTypeNoOperator). */
final class Operators extends SyntacticRule("Operators"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case t: Type.ApplyInfix => Patch.lint(Diagnostic("op", t.op.value, t.pos)) }.asPatch
