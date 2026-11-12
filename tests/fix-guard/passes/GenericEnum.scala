package rules
import teq.fix.*
enum Outcome[+T]:
  case Good(value: T)
  case Bad(why: String)
final class GenericEnum extends SyntacticRule("GenericEnum"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (Outcome.Good(1)).toString, doc.tree.pos))
