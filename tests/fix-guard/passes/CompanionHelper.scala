package rules
import teq.fix.*
final class CompanionHelper extends SyntacticRule("CompanionHelper"):
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
object CompanionHelper:
  def count(text: String): Int =
    var n = 0
    for c <- text do if c == 'a' then n += 1
    n
