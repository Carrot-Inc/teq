package rules
import teq.fix.*
enum Level extends Ordered[Level]:
  def compare(that: Level): Int = ordinal - that.ordinal
  case Low, High
final class EnumOrdered extends SyntacticRule("EnumOrdered"):
  private val floor: Level = Level.Low
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (floor.ordinal).toString, doc.tree.pos))
