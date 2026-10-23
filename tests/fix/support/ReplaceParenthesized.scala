import teq.fix.*

/** Replaces an infix pattern over a parenthesized operand, a refinement's body after a parenthesized parent and a named
  * tuple's field of a parenthesized value, each by its extent (tests/fix/Driver/ParenthesizedOperands). */
final class ReplaceParenthesized extends SyntacticRule("ReplaceParenthesized"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect {
      case p: Pat.ExtractInfix if p.syntax.contains(":: (") => Patch.replaceTree(p, "h :: t")
      case r: Type.Refine if !r.body.syntax.contains("=") => Patch.replaceTree(r.body, "{ type T = Int }")
      case a: Term.Assign if a.syntax.contains("= (") => Patch.replaceTree(a, "a = 2")
    }.asPatch
