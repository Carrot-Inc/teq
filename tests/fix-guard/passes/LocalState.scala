package rules
import teq.fix.*
final class LocalState extends SyntacticRule("LocalState"):
  override def fix(using doc: SyntacticDocument): Patch = { val buffer = scala.collection.mutable.ListBuffer.empty[Patch]; var i = 0; while i < 2 do { i += 1 }; buffer.toList.asPatch }
