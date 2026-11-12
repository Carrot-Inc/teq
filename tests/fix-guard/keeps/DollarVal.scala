package rules
import teq.fix.*
final class DollarVal extends SyntacticRule("DollarVal"):
  private val seen$ = scala.collection.mutable.Set.empty[String]
  override def fix(using doc: SyntacticDocument): Patch =
    seen$ += doc.input.path
    Patch.lint(Diagnostic("n", (seen$.size).toString, doc.tree.pos))
