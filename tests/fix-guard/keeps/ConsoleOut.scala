package rules
import teq.fix.*
final class ConsoleOut extends SyntacticRule("ConsoleOut"):
  override def fix(using doc: SyntacticDocument): Patch = { Console.err.println("x"); Patch.empty }
