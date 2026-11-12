package rules
import teq.fix.*
final class VarInsideLambda extends SyntacticRule("VarInsideLambda"):
  val bump: Int => Int = x => { var y = x; y += 1; y }
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
