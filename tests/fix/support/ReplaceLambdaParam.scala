import teq.fix.*

/** Replaces each lambda parameter whose type is written in parentheses with `x: Int`, whole
  * (tests/fix/Driver/ReplaceLambdaParam). */
final class ReplaceLambdaParam extends SyntacticRule("ReplaceLambdaParam"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect {
      case p: Term.Param if p.parent.exists(_.parent.exists(_.isInstanceOf[Term.Function])) && p.syntax.contains("(") => Patch.replaceTree(p, "x: Int")
    }.asPatch
