import teq.fix.*

/** Removes the importees `List` and `Vector`, each as an atomic patch of its own (tests/fix/Suppression/AtomicIndependent,
  * the review of `70fa37159`). */
final class RemoveListVectorAtomic extends SyntacticRule("RemoveListVectorAtomic"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case i: Importee.Name if i.syntax == "List" || i.syntax == "Vector" => Patch.removeImportee(i).atomic }.asPatch
