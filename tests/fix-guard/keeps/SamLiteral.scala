package rules
import teq.fix.*
object Matchers:
  def make(): SymbolMatcher =
    var n = 0
    sym => { n += 1; n % 3 == 0 }
final class SamLiteral extends SyntacticRule("SamLiteral"):
  private val counting: SymbolMatcher = Matchers.make()
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (counting.matches(Symbol("a/B#"))).toString, doc.tree.pos))
