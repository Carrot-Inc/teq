package rules
import teq.fix.*
case class Counter(start: Int):
  def next(): Int = start
object Makers:
  def make(): Counter =
    var n = 0
    new Counter(0):
      override def next(): Int = { n += 1; n }
final class CaseSubclass extends SyntacticRule("CaseSubclass"):
  private val counter: Counter = Makers.make()
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (counter.next()).toString, doc.tree.pos))
