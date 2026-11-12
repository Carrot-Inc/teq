package rules
import teq.fix.*
final class VarMember extends SyntacticRule("VarMember"):
  var n = 0
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
