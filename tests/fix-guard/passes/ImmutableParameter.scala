package rules
import teq.fix.*
final class ImmutableParameter(words: List[String]) extends SyntacticRule("ImmutableParameter"):
  def this() = this(Nil)
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
