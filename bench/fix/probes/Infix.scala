import teq.fix.*
final class Infix extends SyntacticRule("Infix"):
  override def fix(using doc: SyntacticDocument): Patch =
    val n = doc.tree.collect { case Term.ApplyInfix.Initial(_, Term.Name("=="), _, _) => 1 }.size
    Patch.empty
