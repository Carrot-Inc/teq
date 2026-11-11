// The class's own given beats the companion's for the right-hand side of a val that takes its overridden implicit's
// type, and for a member after it: the class context is whole while the val's right-hand side is typed
// (Namer.inferredResultType; scalac 3.8.4 prints ex ex).
class Foo(val s: String)
object Foo:
  given Foo = Foo("companion")
trait Ex:
  implicit val label: String
  given foo: Foo = Foo("ex")
  def a: Foo
object Main:
  def run(): String =
    val e = new Ex:
      val label = summon[Foo].s
      val a: Foo = summon[Foo]
    e.label + " " + e.a.s
  def main(args: Array[String]): Unit = println(run())
