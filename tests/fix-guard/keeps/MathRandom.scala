package rules
import teq.fix.*
final class MathRandom extends SyntacticRule("MathRandom"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", ((math.random() * 1000).toInt).toString, doc.tree.pos))
