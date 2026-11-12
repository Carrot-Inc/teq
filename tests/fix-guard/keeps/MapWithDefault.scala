package rules
import teq.fix.*
object Makers:
  def make(): Map[String, Int] =
    var n = 0
    Map.empty[String, Int].withDefault(_ => { n += 1; n })
final class MapWithDefault extends SyntacticRule("MapWithDefault"):
  private val next: Map[String, Int] = Makers.make()
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (next("x")).toString, doc.tree.pos))
