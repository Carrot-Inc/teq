import teq.fix.*

/** Each parameter's name by its kind and value, as scalameta's: an anonymous one's a `Name.Anonymous`, its value empty
  * (tests/fix/Driver/AnonymousParams). */
final class ParamNames extends SyntacticRule("ParamNames"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case p: Term.Param =>
      Patch.lint(Diagnostic("name", p.name.kind + " `" + p.name.value + "`", p.pos))
    }.asPatch
