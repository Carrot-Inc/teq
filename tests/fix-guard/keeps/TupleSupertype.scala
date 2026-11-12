package rules
import teq.fix.*
final class TupleSupertype extends SyntacticRule("TupleSupertype"):
  private val pair: Tuple = (scala.collection.mutable.Set.empty[String], 1)
  override def fix(using doc: SyntacticDocument): Patch =
    val n = pair match
      case (s: scala.collection.mutable.Set[String] @unchecked, _) => s += doc.input.path; s.size
      case _ => -1
    Patch.lint(Diagnostic("n", (n).toString, doc.tree.pos))
