package rules
import teq.fix.*
final class ArrayField extends SyntacticRule("ArrayField"):
  val cells = new Array[Int](4)
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
