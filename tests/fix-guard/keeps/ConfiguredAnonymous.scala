package rules
import teq.fix.*
final class ConfiguredAnonymous extends SyntacticRule("ConfiguredAnonymous"):
  override def withConfiguration(c: Configuration): Configured[Rule] =
    var n = 0
    Configured.Ok(new SyntacticRule("ConfiguredAnonymous") {
      override def fix(using doc: SyntacticDocument): Patch =
        n += 1
        Patch.lint(Diagnostic("n", (n).toString, doc.tree.pos))
    })
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.empty
