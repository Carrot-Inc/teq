import teq.fix.*
final class ApplySel extends SyntacticRule("ApplySel"):
  override def fix(using doc: SyntacticDocument): Patch =
    val n = doc.tree.collect { case Term.Apply.Initial(Term.Select(_, Term.Name("==")), _) => 1 }.size
    Patch.empty
