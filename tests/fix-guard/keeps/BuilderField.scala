package rules
import teq.fix.*
final class BuilderField extends SyntacticRule("BuilderField"):
  val text = new StringBuilder
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
