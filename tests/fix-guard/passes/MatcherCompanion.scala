package rules
import teq.fix.*
final class MatcherCompanion extends SemanticRule("MatcherCompanion"):
  override def fix(using doc: SemanticDocument): Patch =
    doc.tree.collect { case MatcherCompanion.get(t) => Patch.lint(Diagnostic("n", "g", t.pos)) }.asPatch
object MatcherCompanion:
  val get: SymbolMatcher = SymbolMatcher.normalized("scala.Option.get")
  private val both = get + SymbolMatcher.exact("a/B#")
