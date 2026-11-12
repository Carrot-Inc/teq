package rules
import teq.fix.*
object Matchers:
  def make(): SymbolMatcher =
    var n = 0
    new SymbolMatcher:
      def matches(sym: Symbol): Boolean = { n += 1; n % 2 == 0 }
      override def toString: String = n.toString
  def last(m: SymbolMatcher): String = m.toString
final class SamMatcher extends SyntacticRule("SamMatcher"):
  private val counting: SymbolMatcher = Matchers.make()
  override def fix(using doc: SyntacticDocument): Patch =
    counting.matches(Symbol("a/B#"))
    Patch.lint(Diagnostic("n", (Matchers.last(counting)).toString, doc.tree.pos))
