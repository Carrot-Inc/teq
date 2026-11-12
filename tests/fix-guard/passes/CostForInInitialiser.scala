package rules
import teq.fix.*
final class CostForInInitialiser extends SyntacticRule("CostForInInitialiser"):
  private val squares: List[Int] = for i <- List(1, 2, 3) yield i * i
  private val keyed: Map[String, Int] = List("a", "b").zipWithIndex.toMap
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (doc.input.path.length).toString, doc.tree.pos))
