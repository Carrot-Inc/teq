package rules
import teq.fix.*
final class CostCompanionConstants extends SyntacticRule("CostCompanionConstants"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (CostCompanionConstants.Names.size).toString, doc.tree.pos))
object CostCompanionConstants:
  val Names: Set[String] = Set("a")
  private val Shape = "a+".r
  final val Max = 3
