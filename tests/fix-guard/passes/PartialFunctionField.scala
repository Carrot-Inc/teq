package rules
import teq.fix.*
final class PartialFunctionField extends SyntacticRule("PartialFunctionField"):
  val pick: PartialFunction[String, Int] = { case "a" => 1 }
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
