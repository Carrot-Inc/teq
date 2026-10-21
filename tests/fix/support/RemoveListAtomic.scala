import teq.fix.*

/** Removes the importee `List`, as an atomic patch, whose edit as applied reaches to the next selector
  * (tests/fix/Suppression/AtomicComments*, the review of `4fd7f3293`). */
final class RemoveListAtomic extends SyntacticRule("RemoveListAtomic"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case i: Importee.Name if i.syntax == "List" => Patch.removeImportee(i).atomic }.asPatch
