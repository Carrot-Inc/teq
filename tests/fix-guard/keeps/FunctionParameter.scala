package rules
import teq.fix.*
object Sources:
  def make(): () => Int =
    var n = 0
    () => { n += 1; n }
final class FunctionParameter(next: () => Int) extends SyntacticRule("FunctionParameter"):
  def this() = this(Sources.make())
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
