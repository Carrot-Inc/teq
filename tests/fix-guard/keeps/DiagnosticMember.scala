package rules
import teq.fix.*
object Makers:
  def make(): Diagnostic =
    var n = 0
    new Diagnostic:
      def message: String = "m"
      def position: Position = null
      override def explanation: String = { n += 1; n.toString }
final class DiagnosticMember extends SyntacticRule("DiagnosticMember"):
  private val ticking: Diagnostic = Makers.make()
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (ticking.explanation).toString, doc.tree.pos))
