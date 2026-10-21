import teq.fix.*

/** Replaces the whole source and adds an import at its start: two patches at offset zero, the replacement made first,
  * which `Rewrites.Patches.apply`'s stable sort by start keeps first and refuses as overlapping
  * (tests/fix/Driver/TieOrder). */
final class ReplaceAndImport extends SyntacticRule("ReplaceAndImport"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.replaceTree(doc.tree, "object Replaced\n") + Patch.addGlobalImport("scala.collection.mutable")
