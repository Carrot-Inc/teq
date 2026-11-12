package rules
import teq.fix.*
final class FreshRandom extends SyntacticRule("FreshRandom"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (new java.util.Random().nextInt(1000)).toString, doc.tree.pos))
