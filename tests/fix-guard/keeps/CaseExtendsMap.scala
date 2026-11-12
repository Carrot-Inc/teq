package rules
import teq.fix.*
case class Bag(k: Int) extends scala.collection.mutable.HashMap[String, Int]
final class CaseExtendsMap extends SyntacticRule("CaseExtendsMap"):
  private val bag = Bag(1)
  override def fix(using doc: SyntacticDocument): Patch =
    bag(doc.input.path) = 1
    Patch.lint(Diagnostic("n", (bag.size).toString, doc.tree.pos))
