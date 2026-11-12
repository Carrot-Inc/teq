package rules
import teq.fix.*
final class ByNameParameter(next: => Int) extends SyntacticRule("ByNameParameter"):
  def this() = this(0)
  override def withConfiguration(c: Configuration): Configured[Rule] =
    var n = 0
    Configured.Ok(new ByNameParameter({ n += 1; n }))
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (next).toString, doc.tree.pos))
