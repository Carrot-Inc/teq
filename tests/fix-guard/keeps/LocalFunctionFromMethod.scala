package rules
import teq.fix.*
object Builders:
  def make(): () => Int =
    var n = 0
    () => { n += 1; n }
final class LocalFunctionFromMethod extends SyntacticRule("LocalFunctionFromMethod"):
  private val next: () => Int =
    val g = Builders.make()
    () => g()
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
