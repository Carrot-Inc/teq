package rules
import teq.fix.*
class Cell:
  var x = 0
final class PlainClassField extends SyntacticRule("PlainClassField"):
  val cell = new Cell
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
