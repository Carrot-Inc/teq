package rules
import teq.fix.*
final class InnerCaseClass extends SyntacticRule("InnerCaseClass"):
  private case class Hit(name: String, at: Int)
  private val none: Option[Hit] = None
  private object Names { val all: Set[String] = Set("a") }
  private sealed trait Kind
  private case object Plain extends Kind
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (Hit("a", 1).name + Names.all.size + none.size).toString, doc.tree.pos))
