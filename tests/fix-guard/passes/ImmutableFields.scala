package rules
import teq.fix.*
final class ImmutableFields extends SyntacticRule("ImmutableFields"):
  val words: Set[String] = Set("a")
  val sizes: Map[String, Int] = Map("a" -> 1)
  val first: Option[String] = None
  val pair: (String, Int) = ("a", 1)
  val all: Vector[Int] = Vector(1)
  val names: List[String] = List("a")
  val shape = "a+".r
  val n = 3
  private val s = "x"
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
