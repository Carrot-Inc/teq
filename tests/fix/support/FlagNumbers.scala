import teq.fix.*

/** A lint at each literal `1` and `2`. */
final class FlagNumbers extends SyntacticRule("FlagNumbers"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case n: Lit.Int if n.value == 1 || n.value == 2 => Patch.lint(Diagnostic("number", "number", n.pos)) }.asPatch
