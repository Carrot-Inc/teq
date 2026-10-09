// Type arguments of a parent class can make two alternatives of an overloaded name one method
// in a subclass: `handle(t: T)` of the trait and `handle(t: String)` of the class are both
// `handle(String)` in `Named`, and a call through the trait reaches the class's method.
trait Handler[T]:
  def handle(t: T): String
  def run(t: T): String = "ran " + handle(t)

abstract class Base[T] extends Handler[T]:
  def handle(t: String): String = "base " + t

class Named extends Base[String]

class Counted extends Base[Int]:
  def handle(t: Int): String = "counted " + t

@main def run(): Unit =
  println(Named().run("x"))
  println(Named().handle("y"))
  println(Counted().run(1))
  println(Counted().handle("z"))
  println(Counted().handle(2))
