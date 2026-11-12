package rules
import teq.fix.*
final class MatcherField extends SemanticRule("MatcherField"):
  val get = SymbolMatcher.exact("scala/Option#get().")
  override def fix(using doc: SemanticDocument): Patch = Patch.empty
