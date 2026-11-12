package rules
import teq.fix.*
final class AnonymousObject extends SyntacticRule("AnonymousObject"):
  val cell = new Object { var n = 0 }
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
