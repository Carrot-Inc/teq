package rules
import teq.fix.*
object Tally:
  var n = 0
final class ObjectVar extends SyntacticRule("ObjectVar"):
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
