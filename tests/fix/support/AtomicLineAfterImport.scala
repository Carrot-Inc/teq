import teq.fix.*

/** The line break after each import removed, an atomic patch. */
final class AtomicLineAfterImport extends SyntacticRule("AtomicLineAfterImport"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case i: Import =>
      doc.tokens.toList.find(t => t.pos.start >= i.pos.end && t.kind == "LF").map(Patch.removeToken).asPatch.atomic
    }.asPatch
