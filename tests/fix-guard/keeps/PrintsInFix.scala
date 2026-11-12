package rules
import teq.fix.*
final class PrintsInFix extends SyntacticRule("PrintsInFix"):
  override def fix(using doc: SyntacticDocument): Patch = { println(doc.input.path); Patch.empty }
