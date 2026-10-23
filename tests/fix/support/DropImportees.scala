import teq.fix.*

/** A line before `import Names.a` and its importee removed (the pass over 0f91053d7's `Drop`, of one import). */
final class DropImportees extends SyntacticRule("DropImportees"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case i: Import if i.syntax == "import Names.a" =>
      Patch.addLeft(i, "val replacement = 1\n") + i.importers.flatMap(_.importees).map(Patch.removeImportee).asPatch
    }.asPatch
