package rules
import teq.fix.*
object Makers:
  def make(): () => Int =
    var n = 0
    () => { n += 1; n }
final class OptionOfFunction extends SyntacticRule("OptionOfFunction"):
  val next: Option[() => Int] = Some(Makers.make())
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
