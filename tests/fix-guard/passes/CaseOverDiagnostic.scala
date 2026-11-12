package rules
import teq.fix.*
case class Violation(diagnostic: Diagnostic, patch: Patch)
final class CaseOverDiagnostic extends SyntacticRule("CaseOverDiagnostic"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (1).toString, doc.tree.pos))
