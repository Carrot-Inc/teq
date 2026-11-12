package rules
import teq.fix.*
final class LazyBlockVar extends SyntacticRule("LazyBlockVar"):
  lazy val x: Int = { var n = 0; n += 1; n }
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
