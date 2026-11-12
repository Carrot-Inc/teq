package rules
import teq.fix.*
trait Holding[S]:
  def make(): S
  val held: S = make()
final class GenericTrait extends SyntacticRule("GenericTrait"), Holding[scala.collection.mutable.Set[String]]:
  def make(): scala.collection.mutable.Set[String] = scala.collection.mutable.Set.empty[String]
  override def fix(using doc: SyntacticDocument): Patch =
    held += doc.input.path
    Patch.lint(Diagnostic("n", (held.size).toString, doc.tree.pos))
