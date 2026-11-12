package rules
import teq.fix.*
final class NoMembers extends SyntacticRule("NoMembers"):
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
