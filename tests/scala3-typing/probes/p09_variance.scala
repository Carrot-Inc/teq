class Foo[+A]:
  def foo(a: A): Unit = {}
class Bar[-M](val v: List[M])
trait Box[+A]:
  def get: A
  def set(a: A): Unit
class Cell[+A](var x: A)
trait Sink[-A]:
  def get: A
@main def run(): Unit = println(1)
