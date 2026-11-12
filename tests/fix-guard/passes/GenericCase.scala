package rules
import teq.fix.*
case class Located[T](value: T, at: Int)
final class GenericCase extends SyntacticRule("GenericCase"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (Located("a", 1).value).toString, doc.tree.pos))
