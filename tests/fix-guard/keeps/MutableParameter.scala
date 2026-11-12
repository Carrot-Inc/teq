package rules
import teq.fix.*
final class MutableParameter(buffer: scala.collection.mutable.ListBuffer[String]) extends SyntacticRule("MutableParameter"):
  def this() = this(scala.collection.mutable.ListBuffer.empty[String])
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
