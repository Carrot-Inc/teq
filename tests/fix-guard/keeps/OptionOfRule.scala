package rules
import teq.fix.*
object Makers:
  def make(): SyntacticRule =
    var n = 0
    new SyntacticRule("Inner"):
      override def fix(using doc: SyntacticDocument): Patch =
        n += 1
        Patch.lint(Diagnostic("n", (n).toString, doc.tree.pos))
final class OptionOfRule extends SyntacticRule("OptionOfRule"):
  private val inner: Option[SyntacticRule] = Some(Makers.make())
  override def fix(using doc: SyntacticDocument): Patch =
    inner.get.fix
