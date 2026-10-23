import teq.fix.*

/** Reports each typed parameter's name (tests/fix/Driver/TypedParamNames). */
final class TypedNames extends SyntacticRule("TypedNames"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case t: Type.TypedParam => Patch.lint(Diagnostic("name", t.name.value, t.pos)) }.asPatch
