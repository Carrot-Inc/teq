package rules
import teq.fix.*
object Store:
  val all = scala.collection.mutable.ListBuffer.empty[String]
final class ObjectWithBuffer extends SyntacticRule("ObjectWithBuffer"):
  val d = Store
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
