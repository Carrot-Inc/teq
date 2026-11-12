package rules
import teq.fix.*
object ByStart extends Ordering[Token]:
  def compare(a: Token, b: Token): Int = a.start - b.start
final class OrderingObject extends SyntacticRule("OrderingObject"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (doc.tokens.toList.sorted(using ByStart).size).toString, doc.tree.pos))
