package rules
import teq.fix.*
object Zero extends Ordered[Int]:
  def compare(that: Int): Int = -that
final class ObjectOrdered extends SyntacticRule("ObjectOrdered"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (Zero.compare(1)).toString, doc.tree.pos))
