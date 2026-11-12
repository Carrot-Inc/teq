package rules
import teq.fix.*
final class VarInsideNestedDef extends SyntacticRule("VarInsideNestedDef"):
  val f: () => Int =
    def g(): Int = { var n = 0; n += 1; n }
    () => g()
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
