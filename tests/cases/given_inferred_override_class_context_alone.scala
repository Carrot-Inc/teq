// given_inferred_override_class_context with no companion given: the class's own given is the only one
// (scalac 3.8.4 prints ex).
class Foo(val s: String)
trait Ex:
  implicit val label: String
  given foo: Foo = Foo("ex")
object Main:
  def run(): String =
    val e = new Ex:
      val label = summon[Foo].s
    e.label
  def main(args: Array[String]): Unit = println(run())
