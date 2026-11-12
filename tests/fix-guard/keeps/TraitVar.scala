package rules
import teq.fix.*
trait Counting:
  var n = 0
final class TraitVar extends SyntacticRule("TraitVar"), Counting:
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
