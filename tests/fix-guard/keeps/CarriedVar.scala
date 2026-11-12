package rules
import teq.fix.*
final class CarriedVar extends SyntacticRule("CarriedVar"):
  private val next: () => Int =
    var n = 0
    () => { n += 1; n }
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
