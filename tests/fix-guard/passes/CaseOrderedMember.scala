package rules
import teq.fix.*
case class Version(major: Int, minor: Int) extends Ordered[Version]:
  def compare(that: Version): Int = if major != that.major then major - that.major else minor - that.minor
final class CaseOrderedMember extends SyntacticRule("CaseOrderedMember"):
  private val floor: Version = Version(2, 13)
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (Version(3, 0) > floor).toString, doc.tree.pos))
