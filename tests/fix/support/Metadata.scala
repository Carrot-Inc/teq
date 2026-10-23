import teq.fix.*

/** scalafix's `Rule` metadata: this rule's, `isRewrite` and `isExperimental` overridden and `isLinter` the default, and
  * the shipped DisableSyntax's, a linter as its source declares; a lint at the token `marker`. */
final class Metadata extends SyntacticRule("Metadata"):
  override def isRewrite: Boolean = true
  override def isExperimental: Boolean = true
  override def fix(using doc: SyntacticDocument): Patch =
    val at = doc.tokens.find(_.text == "marker").map(_.pos).getOrElse(doc.tree.pos)
    def flags(r: Rule): String = s"${r.name} linter=${r.isLinter} rewrite=${r.isRewrite} experimental=${r.isExperimental}"
    Patch.lint(Diagnostic("metadata", flags(this) + "; " + flags(new rules.DisableSyntax()), at))
