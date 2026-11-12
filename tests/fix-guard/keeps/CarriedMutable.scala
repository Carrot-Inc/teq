package rules
import teq.fix.*
final class CarriedMutable extends SyntacticRule("CarriedMutable"):
  private val seen: String => Boolean =
    val all = scala.collection.mutable.Set.empty[String]
    s => all.add(s)
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
