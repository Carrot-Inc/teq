package rules
import teq.fix.*
final class PatternMutable extends SyntacticRule("PatternMutable"):
  val (a, b) = (1, scala.collection.mutable.Set.empty[Int])
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
