package rules
import teq.fix.*
final class FunctionField extends SyntacticRule("FunctionField"):
  val isBad: String => Boolean = s => s.startsWith("x")
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
