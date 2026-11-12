package rules
import teq.fix.*
case class Box[T](value: T)
final class WildCase extends SyntacticRule("WildCase"):
  private val box: Box[?] = Box(scala.collection.mutable.Set.empty[String])
  override def fix(using doc: SyntacticDocument): Patch =
    box.value match
      case s: scala.collection.mutable.Set[String] @unchecked =>
        s += doc.input.path
        Patch.lint(Diagnostic("n", (s.size).toString, doc.tree.pos))
      case _ => Patch.empty
