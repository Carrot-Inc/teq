package rules
import teq.fix.*
final class BacktickDollar extends SyntacticRule("BacktickDollar"):
  private var `my$n` = 0
  override def fix(using doc: SyntacticDocument): Patch =
    `my$n` += 1
    Patch.lint(Diagnostic("n", (`my$n`).toString, doc.tree.pos))
