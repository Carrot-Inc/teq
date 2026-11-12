package rules
import teq.fix.*
case class Box(cells: Array[Int])
final class CaseClassWithArray extends SyntacticRule("CaseClassWithArray"):
  val box = Box(Array(1))
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
