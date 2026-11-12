package rules
import teq.fix.*
final class CostNestedObject extends SyntacticRule("CostNestedObject"):
  private object Names:
    val all: Set[String] = Set("a")
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (Names.all.size).toString, doc.tree.pos))
