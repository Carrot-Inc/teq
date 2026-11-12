package rules
import teq.fix.*
final class DollarVar extends SyntacticRule("DollarVar"):
  private var n$ = 0
  override def fix(using doc: SyntacticDocument): Patch =
    n$ += 1
    Patch.lint(Diagnostic("n", (n$).toString, doc.tree.pos))
