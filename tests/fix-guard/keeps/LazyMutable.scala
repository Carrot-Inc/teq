package rules
import teq.fix.*
final class LazyMutable extends SyntacticRule("LazyMutable"):
  lazy val seen = scala.collection.mutable.Set.empty[String]
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
