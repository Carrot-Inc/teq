package rules
import teq.fix.*
object Named:
  def unapply(t: Tree): Option[String] = t match
    case n: Term.Name => Some(n.syntax)
    case _ => None
final class CostExtractorObject extends SyntacticRule("CostExtractorObject"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case Named(n) => Patch.lint(Diagnostic("n", (n).toString, doc.tree.pos)) }.asPatch
