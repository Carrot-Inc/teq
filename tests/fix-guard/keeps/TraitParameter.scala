package rules
import teq.fix.*
trait Kept[S](val held: S)
final class TraitParameter extends SyntacticRule("TraitParameter"), Kept[scala.collection.mutable.Set[String]](scala.collection.mutable.Set.empty[String]):
  override def fix(using doc: SyntacticDocument): Patch =
    held += doc.input.path
    Patch.lint(Diagnostic("n", (held.size).toString, doc.tree.pos))
