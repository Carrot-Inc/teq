package rules
import teq.fix.*
final class GeneralIterable extends SyntacticRule("GeneralIterable"):
  val items: Iterable[Int] = scala.collection.mutable.ListBuffer(1)
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
