import teq.fix.*

/** Replaces each `Term.Annotate` whose expression is another with that expression's text: an outer ascription removed,
  * the inner kept (tests/fix/Driver/UnwrapNestedAnnotate). */
final class UnwrapAnnotate extends SyntacticRule("UnwrapAnnotate"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect {
      case t: Term.Annotate if t.expr.isInstanceOf[Term.Annotate] => Patch.replaceTree(t, t.expr.syntax)
    }.asPatch
