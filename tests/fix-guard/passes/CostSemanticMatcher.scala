package rules
import teq.fix.*
final class CostSemanticMatcher extends SemanticRule("CostSemanticMatcher"):
  private val get = SymbolMatcher.exact("scala/Option#get().")
  private val both = get + SymbolMatcher.normalized("scala/Some#get")
  override def fix(using doc: SemanticDocument): Patch = Patch.empty
