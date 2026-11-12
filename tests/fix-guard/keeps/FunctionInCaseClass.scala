package rules
import teq.fix.*
case class Holder(next: () => Int)
object Counters:
  def make(): () => Int =
    var n = 0
    () => { n += 1; n }
final class FunctionInCaseClass extends SyntacticRule("FunctionInCaseClass"):
  val holder = Holder(Counters.make())
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
