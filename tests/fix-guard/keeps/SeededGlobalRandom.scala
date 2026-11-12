package rules
import teq.fix.*
final class SeededGlobalRandom extends SyntacticRule("SeededGlobalRandom"):
  override def withConfiguration(c: Configuration): Configured[Rule] =
    scala.util.Random.setSeed(42L)
    Configured.Ok(this)
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (scala.util.Random.nextInt(1000)).toString, doc.tree.pos))
