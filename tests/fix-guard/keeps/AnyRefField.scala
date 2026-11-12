package rules
import teq.fix.*
final class AnyRefField extends SyntacticRule("AnyRefField"):
  val thing: AnyRef = new Object
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
