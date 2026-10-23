import teq.fix.*

/** Reports each string literal's and interpolation part's value as UTF-16 units with its length, through `String`'s own
  * members, and each char literal's unit, as scalameta's `Lit.String.value` and `Lit.Char.value` give them, a lone
  * surrogate's among them (tests/fix/Driver/LiteralUnits). */
final class LiteralUnits extends SyntacticRule("LiteralUnits"):
  override def fix(using doc: SyntacticDocument): Patch =
    def units(s: String) = (0 until s.length).map(i => s.charAt(i).toInt).mkString(" ")
    doc.tree.collect {
      case t: Lit.String => Patch.lint(Diagnostic("string", s"length ${t.value.length}: ${units(t.value)}", t.pos, severity = LintSeverity.Warning))
      case t: Lit.Char => Patch.lint(Diagnostic("char", t.value.toInt.toString, t.pos, severity = LintSeverity.Warning))
    }.asPatch
