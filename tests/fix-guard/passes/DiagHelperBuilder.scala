package rules
import teq.fix.*
final class DiagHelperBuilder extends SyntacticRule("DiagHelperBuilder"):
  private def report(t: Tree): Diagnostic =
    val sb = new StringBuilder
    sb.append("found ").append(t.syntax.length)
    Diagnostic("found", sb.toString, t.pos)
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(report(doc.tree))
