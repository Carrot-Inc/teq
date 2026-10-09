// An array overload added in a subclass beside an override: the override, what it overrides and
// a sibling subclass's override are called through the base alike.
class A:
  def f(x: Array[Int]): Int = 1
class B extends A:
  override def f(x: Array[Int]): Int = 2
  def f(x: Array[String]): Int = 3
class D extends A:
  override def f(x: Array[Int]): Int = 4

@main def run(): Unit =
  println((new B: A).f(Array(0)))
  println((new B).f(Array("s")))
  println((new D: A).f(Array(0)))
  println(new A().f(Array(0)))
