package rules
import teq.fix.*
final class StringArrayField extends SyntacticRule("StringArrayField"):
  val names: Array[String] = Array("a")
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
