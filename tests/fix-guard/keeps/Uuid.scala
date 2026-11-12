package rules
import teq.fix.*
final class Uuid extends SyntacticRule("Uuid"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (java.util.UUID.randomUUID().toString.take(8)).toString, doc.tree.pos))
