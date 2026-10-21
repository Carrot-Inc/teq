import teq.fix.*
final class TokList extends SyntacticRule("TokList"):
  override def fix(using doc: SyntacticDocument): Patch =
    val n = doc.tokens.toList.size
    Patch.empty
