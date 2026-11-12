package rules
import teq.fix.*
final class DiagHelperVar extends SyntacticRule("DiagHelperVar"):
  private def report(t: Tree, names: List[String]): Diagnostic =
    var msg = "found"
    if names.nonEmpty then msg += names.mkString(": ", ", ", "")
    Diagnostic("found", msg, t.pos)
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(report(doc.tree, List("a")))
