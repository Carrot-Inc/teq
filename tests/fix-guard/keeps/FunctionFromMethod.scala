package rules
import teq.fix.*
final class FunctionFromMethod extends SyntacticRule("FunctionFromMethod"):
  private val next: () => Int = make()
  private def make(): () => Int =
    var n = 0
    () => { n += 1; n }
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
