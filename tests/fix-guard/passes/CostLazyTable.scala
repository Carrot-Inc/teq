package rules
import teq.fix.*
final class CostLazyTable extends SyntacticRule("CostLazyTable"):
  private lazy val table: Map[String, Int] = List("a", "b").zipWithIndex.toMap
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (doc.input.path.length).toString, doc.tree.pos))
