import teq.fix.*

/** Adds `import scala.collection.immutable` as an atomic patch, which a suppression of the rule at the token it is
  * added beside drops whole (tests/fix/Suppression/AtomicImport). */
final class AddImmutable extends SyntacticRule("AddImmutable"):
  override def fix(using doc: SyntacticDocument): Patch = Patch.addGlobalImport("scala.collection.immutable").atomic
