package rules
import teq.fix.*
final class GlobalRandom extends SyntacticRule("GlobalRandom"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (scala.util.Random.nextInt(1000)).toString, doc.tree.pos))
