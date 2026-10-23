import teq.fix.*

/** Replaces each lambda, case, generator and guard written with a closing parenthesis last, whole: the file types after
  * the edits as it would were that parenthesis outside the replaced span (tests/fix/Driver/ParenthesizedBodies). */
final class ReplaceBodies extends SyntacticRule("ReplaceBodies"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect {
      case t: Term.Function if closed(t) => Patch.replaceTree(t, "identity[Int]")
      case t: Case if closed(t) => Patch.replaceTree(t, "case _ => 0")
      case t: Enumerator.Generator if closed(t) => Patch.replaceTree(t, "x <- 2 :: Nil")
      case t: Enumerator.Guard if closed(t) => Patch.replaceTree(t, "if x > 1")
    }.asPatch

  private def closed(t: Tree): Boolean = t.syntax.endsWith(")")
