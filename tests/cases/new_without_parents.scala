// `new:` and `new { ... }` without a parent clause take the expected type as their parent, a trait
// or a class whose constructor takes no arguments (dotty's typedNew); with no such type, AnyRef.
trait Reader[A]:
  def read(s: String): A
  def name: String = "reader"

abstract class Base:
  def greet: String
  override def toString = s"Base($greet)"

class Counter:
  var n = 0

given Reader[Int] = new:
  def read(s: String) = s.length

def describe(r: Reader[String]): String = r.read("abc") + "/" + r.name

@main def run(): Unit =
  println(summon[Reader[Int]].read("hello"))
  println(describe(new:
    def read(s: String) = s.reverse
    override def name = "reversed"
  ))
  val b: Base = new { def greet = "hi" }
  println(b)
  val c: Counter = new:
    n = 5
  println(c.n)
  val plain = new { val x = 1 }
  println(plain.getClass.getName.nonEmpty)
