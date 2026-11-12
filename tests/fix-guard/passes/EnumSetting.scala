package rules
import teq.fix.*
enum Mode:
  case Strict, Loose
final class EnumSetting(mode: Mode) extends SyntacticRule("EnumSetting"):
  def this() = this(Mode.Strict)
  private val byName: Map[String, Mode] = Mode.values.map(m => m.toString -> m).toMap
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (byName.size).toString, doc.tree.pos))
