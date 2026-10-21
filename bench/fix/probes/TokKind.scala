import teq.fix.*
final class TokKind extends SyntacticRule("TokKind"):
  override def fix(using doc: SyntacticDocument): Patch =
    var n = 0
    doc.tokens.toList.foreach(t => if t.kind == "Semicolon" then n += 1)
    Patch.empty
