package rules
import teq.fix.*
final class OptionOfBuffer extends SyntacticRule("OptionOfBuffer"):
  val maybe: Option[scala.collection.mutable.ListBuffer[Int]] = None
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
