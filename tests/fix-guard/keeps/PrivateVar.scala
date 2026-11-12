package rules
import teq.fix.*
final class PrivateVar extends SyntacticRule("PrivateVar"):
  private var n = 0
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
