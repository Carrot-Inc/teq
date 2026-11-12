package rules
import teq.fix.*
object SymbolMatcher:
  def exact(cell: Array[Int]): teq.fix.SymbolMatcher = sym => { cell(0) += 1; cell(0) % 3 == 0 }
final class OwnMatcherObject extends SyntacticRule("OwnMatcherObject"):
  private val counting: teq.fix.SymbolMatcher = SymbolMatcher.exact(new Array[Int](1))
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (counting.matches(Symbol("a/B#"))).toString, doc.tree.pos))
