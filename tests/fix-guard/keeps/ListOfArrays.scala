package rules
import teq.fix.*
final class ListOfArrays extends SyntacticRule("ListOfArrays"):
  val rows: List[Array[Int]] = List(Array(1))
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
