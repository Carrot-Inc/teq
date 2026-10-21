import teq.fix.*
final class Traverse extends SyntacticRule("Traverse"):
  override def fix(using doc: SyntacticDocument): Patch =
    var n = 0
    doc.tree.traverse(_ => n += 1)
    Patch.empty
