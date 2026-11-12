package rules
import teq.fix.*
object Makers:
  def make(): SyntacticRule =
    var n = 0
    new SyntacticRule("Inner"):
      override def fix(using doc: SyntacticDocument): Patch =
        n += 1
        Patch.lint(Diagnostic("n", (n).toString, doc.tree.pos))
final class InnerRule extends SyntacticRule("InnerRule"):
  private val inner: SyntacticRule = Makers.make()
  override def fix(using doc: SyntacticDocument): Patch =
    inner.fix
