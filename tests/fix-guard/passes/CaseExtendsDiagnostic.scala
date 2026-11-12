package rules
import teq.fix.*
final case class TooLong(position: Position, length: Int) extends Diagnostic:
  def message: String = "too long: " + length
final class CaseExtendsDiagnostic extends SyntacticRule("CaseExtendsDiagnostic"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(TooLong(doc.tree.pos, 3))
