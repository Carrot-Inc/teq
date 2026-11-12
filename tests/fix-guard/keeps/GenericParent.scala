package rules
import teq.fix.*
abstract class Keeping[S](val held: S, name: String) extends SyntacticRule(name)
final class GenericParent extends Keeping[scala.collection.mutable.Set[String]](scala.collection.mutable.Set.empty[String], "GenericParent"):
  override def fix(using doc: SyntacticDocument): Patch =
    held += doc.input.path
    Patch.lint(Diagnostic("n", (held.size).toString, doc.tree.pos))
