package rules
import teq.fix.*
final class MatcherSplat extends SemanticRule("MatcherSplat"):
  private val names = List("a.B", "a.C")
  private val any: SymbolMatcher = SymbolMatcher.normalized(names*)
  override def fix(using doc: SemanticDocument): Patch =
    Patch.lint(Diagnostic("n", (any.matches(Symbol("a/B#"))).toString, doc.tree.pos))
