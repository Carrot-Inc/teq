import teq.fix.*
// Every token of the document as a lint: `<kind>|<start>|<end>|<text>` (the bridge's view, compared with Meta.scala's).
final class Dump extends SyntacticRule("Dump"):
  override def fix(using doc: SyntacticDocument): Patch =
    val shown = doc.tokens.toList.map(t => t.kind + "|" + t.start + "|" + t.end + "|" + t.text.replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t").replace("\f", "\\f"))
    Patch.lint(Diagnostic("tokens", "\n" + shown.mkString("\n"), doc.tree.pos))
