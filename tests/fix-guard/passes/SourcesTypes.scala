package rules
import teq.fix.*
case class Limits(max: Int, names: List[String])
enum Mode:
  case Strict, Loose
object Defaults:
  val limit = 3
final class SourcesTypes extends SyntacticRule("SourcesTypes"):
  val limits = Limits(3, List("a"))
  val mode: Mode = Mode.Strict
  val d = Defaults.limit
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
