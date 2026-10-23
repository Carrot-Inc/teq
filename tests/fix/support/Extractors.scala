import teq.fix.*

/** Reports what scalameta's extractors give, through their results: a variable's legacy `rhs`, Some for an initialised
  * one and None for `= _`, and a decimal's format and a string's value, each a `String` (tests/fix/Driver/Extractors). */
final class Extractors extends SyntacticRule("Extractors"):
  override def fix(using doc: SyntacticDocument): Patch =
    def report(id: String, text: String, t: Tree) = Patch.lint(Diagnostic(id, text, t.pos, severity = LintSeverity.Warning))
    doc.tree.collect {
      case v @ Defn.Var(_, _, _, Some(rhs)) => report("initialised", rhs.syntax, v)
      case v @ Defn.Var(_, _, _, None) => report("deferred", "none", v)
      case t @ Lit.Double(s) => report("double", s"$s, length ${s.length}", t)
      case t @ Lit.Float(s) => report("float", s"$s, length ${s.length}", t)
      case t @ Lit.String(s) => report("string", s"$s, length ${s.length}", t)
    }.asPatch
