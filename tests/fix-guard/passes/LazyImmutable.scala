package rules
import teq.fix.*
final class LazyImmutable extends SyntacticRule("LazyImmutable"):
  lazy val words: Set[String] = Set("a", "b")
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
