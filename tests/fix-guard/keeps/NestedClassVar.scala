package rules
import teq.fix.*
final class NestedClassVar extends SyntacticRule("NestedClassVar"):
  class Inner:
    var n = 0
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
