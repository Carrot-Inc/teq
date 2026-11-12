package rules
import teq.fix.*
final class FnPlaceholder extends SyntacticRule("FnPlaceholder"):
  private val short: String => Boolean = _.length < 3
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (short("ab")).toString, doc.tree.pos))
