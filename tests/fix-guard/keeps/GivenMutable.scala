package rules
import teq.fix.*
final class GivenMutable extends SyntacticRule("GivenMutable"):
  given buffer: scala.collection.mutable.ListBuffer[Int] = scala.collection.mutable.ListBuffer.empty[Int]
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
