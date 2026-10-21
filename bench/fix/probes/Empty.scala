import teq.fix.*
final class Empty extends SyntacticRule("Empty"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.empty
