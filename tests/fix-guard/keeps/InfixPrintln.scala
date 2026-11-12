package rules
import teq.fix.*
import java.lang.System.{err => e}
final class InfixPrintln extends SyntacticRule("InfixPrintln"):
  override def withConfiguration(c: Configuration): Configured[Rule] =
    e println "configured"
    Configured.Ok(this)
  override def fix(using doc: SyntacticDocument): Patch =
    e println doc.input.path
    Patch.empty
