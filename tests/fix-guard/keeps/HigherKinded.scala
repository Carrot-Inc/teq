package rules
import teq.fix.*
trait HoldingF[F[_]]:
  def make(): F[String]
  val held: F[String] = make()
final class HigherKinded extends SyntacticRule("HigherKinded"), HoldingF[scala.collection.mutable.Set]:
  def make(): scala.collection.mutable.Set[String] = scala.collection.mutable.Set.empty[String]
  override def fix(using doc: SyntacticDocument): Patch =
    held += doc.input.path
    Patch.lint(Diagnostic("n", (held.size).toString, doc.tree.pos))
