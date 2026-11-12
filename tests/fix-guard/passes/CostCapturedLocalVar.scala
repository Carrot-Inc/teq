package rules
import teq.fix.*
final class CostCapturedLocalVar extends SyntacticRule("CostCapturedLocalVar"):
  override def fix(using doc: SyntacticDocument): Patch =
    var seen = 0
    doc.tokens.foreach(t => if t.text == "val" then seen += 1)
    Patch.lint(Diagnostic("n", (seen).toString, doc.tree.pos))
