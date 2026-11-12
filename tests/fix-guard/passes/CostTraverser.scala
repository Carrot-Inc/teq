package rules
import teq.fix.*
final class CostTraverser extends SyntacticRule("CostTraverser"):
  private def visit(t: Tree, depth: Int): Int = t.children.map(visit(_, depth + 1)).foldLeft(depth)(_ max _)
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (visit(doc.tree, 0)).toString, doc.tree.pos))
