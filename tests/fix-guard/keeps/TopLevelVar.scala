package rules
import teq.fix.*
var hits = 0
final class TopLevelVar extends SyntacticRule("TopLevelVar"):
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
