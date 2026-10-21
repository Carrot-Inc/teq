import teq.fix.*
final class TokIs extends SyntacticRule("TokIs"):
  override def fix(using doc: SyntacticDocument): Patch =
    var n = 0
    doc.tokens.toList.foreach(t => if t.is("Semicolon") || t.is("Tab") || t.is("Ident") then n += 1)
    Patch.empty
