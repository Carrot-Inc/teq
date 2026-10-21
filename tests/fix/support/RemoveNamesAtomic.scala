import teq.fix.*

/** Removes every plain importee, each as an atomic patch, which a suppression of the rule at any token its lowering
  * removes drops whole (tests/fix/Suppression/AtomicRemoval, the review of `b8eb8842a`). */
final class RemoveNamesAtomic extends SyntacticRule("RemoveNamesAtomic"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case i: Importee.Name => Patch.removeImportee(i).atomic }.asPatch
