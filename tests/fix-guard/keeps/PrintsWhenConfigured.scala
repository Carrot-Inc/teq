package rules
import teq.fix.*
final class PrintsWhenConfigured extends SyntacticRule("PrintsWhenConfigured"):
  override def withConfiguration(c: Configuration): Configured[Rule] =
    System.err.println("configured")
    Configured.Ok(this)
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
