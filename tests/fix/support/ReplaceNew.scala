import teq.fix.*

/** Replaces each `Term.New` with `null`, whole (tests/fix/Driver/ReplaceCurriedNew). */
final class ReplaceNew extends SyntacticRule("ReplaceNew"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case n: Term.New => Patch.replaceTree(n, "null") }.asPatch
