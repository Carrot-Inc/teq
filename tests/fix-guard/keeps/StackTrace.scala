package rules
import teq.fix.*
final class StackTrace extends SyntacticRule("StackTrace"):
  override def withConfiguration(c: Configuration): Configured[Rule] =
    new RuntimeException("configured").printStackTrace()
    Configured.Ok(this)
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.empty
