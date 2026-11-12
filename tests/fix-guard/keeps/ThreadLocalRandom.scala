package rules
import teq.fix.*
final class ThreadLocalRandom extends SyntacticRule("ThreadLocalRandom"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (java.util.concurrent.ThreadLocalRandom.current().nextInt(1000)).toString, doc.tree.pos))
