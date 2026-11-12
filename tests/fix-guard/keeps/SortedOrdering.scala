package rules
import teq.fix.*
object Makers:
  def make(): scala.collection.immutable.SortedSet[String] =
    var n = 0
    val counting: Ordering[String] = (a, b) => { n += 1; n }
    scala.collection.immutable.SortedSet.empty[String](using counting)
final class SortedOrdering extends SyntacticRule("SortedOrdering"):
  private val sorted: scala.collection.immutable.SortedSet[String] = Makers.make()
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (sorted.ordering.compare("a", "b")).toString, doc.tree.pos))
