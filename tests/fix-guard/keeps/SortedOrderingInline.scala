package rules
import teq.fix.*
final class SortedOrderingInline extends SyntacticRule("SortedOrderingInline"):
  private val sorted: scala.collection.immutable.SortedSet[String] = (() => { var n = 0; scala.collection.immutable.SortedSet.empty[String](using (a, b) => { n += 1; n }) })()
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (sorted.ordering.compare("a", "b")).toString, doc.tree.pos))
