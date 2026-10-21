import teq.fix.*

/** Every token of the document, as the bridge hands it, a lint each at the token `marker` (one asserted line): its kind,
  * its offsets in UTF-16 units, its text, `is` of its own kind and of a kind no token has, `isKeyword`, `isTrivia` and
  * its value — the token contract the bridge keeps (docs/FIX.md, "Token"). */
final class DumpTokens extends SyntacticRule("DumpTokens"):
  override def fix(using doc: SyntacticDocument): Patch =
    val all = doc.tokens.toList
    all.find(_.text == "marker") match
      case None => Patch.empty
      case Some(at) =>
        def shown(s: String) = s.replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t")
        all.map { t =>
          val line = t.kind + "|" + t.start + "|" + t.end + "|" + shown(t.text) + "|" + t.is(t.kind) + "|" + t.is("NoSuchKind") + "|" + t.isKeyword + "|" + t.isTrivia + "|" + shown(t.value)
          Patch.lint(Diagnostic("token", line, at.pos))
        }.asPatch
