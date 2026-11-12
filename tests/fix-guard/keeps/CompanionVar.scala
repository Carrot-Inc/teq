package rules
import teq.fix.*
final class CompanionVar extends SyntacticRule("CompanionVar"):
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
object CompanionVar:
  var n = 0
