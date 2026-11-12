package rules
import teq.fix.*
case class Cell(var x: Int)
final class CaseClassWithVar extends SyntacticRule("CaseClassWithVar"):
  val box = Cell(1)
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
