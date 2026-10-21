import teq.fix.*
final class Walk extends SyntacticRule("Walk"):
  override def fix(using doc: SyntacticDocument): Patch =
    val n = doc.tree.collect { case t => t }.size
    Patch.empty
