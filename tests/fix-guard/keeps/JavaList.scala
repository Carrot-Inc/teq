package rules
import teq.fix.*
final class JavaList extends SyntacticRule("JavaList"):
  val items = new java.util.ArrayList[String]()
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
