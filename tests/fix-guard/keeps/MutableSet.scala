package rules
import teq.fix.*
final class MutableSet extends SyntacticRule("MutableSet"):
  private val seen = scala.collection.mutable.Set.empty[String]
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
