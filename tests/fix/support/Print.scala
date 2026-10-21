import teq.fix.*

/** A rule whose configuration prints `configured` and whose every document prints `document`: a run that configured
  * its rules twice, or replayed an earlier round, prints more of them (tests/fix.scala's `Typing`, a typing that gave
  * way to one worker and was typed again within its round). */
final class Print extends SyntacticRule("Print"):
  override def withConfiguration(c: Configuration): Configured[Rule] =
    System.err.println("configured")
    Configured.Ok(this)
  override def fix(using doc: SyntacticDocument): Patch =
    System.err.println("document")
    Patch.empty
