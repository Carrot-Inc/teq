package rules
import teq.fix.*
case class Rename[T](from: T, to: T)
final class GenericCaseMember extends SyntacticRule("GenericCaseMember"):
  private val renames: List[Rename[String]] = List(Rename("a", "b"))
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (renames.size).toString, doc.tree.pos))
