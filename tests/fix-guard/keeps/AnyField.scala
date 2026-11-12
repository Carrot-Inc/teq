package rules
import teq.fix.*
final class AnyField extends SyntacticRule("AnyField"):
  val thing: Any = scala.collection.mutable.ListBuffer(1)
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
