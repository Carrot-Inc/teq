// A default argument read while its object initialises another object: completion re-enters through the default.
object A {
  final val a = B.f()
  def f(x: Int = B.c): Int = x
}
object B {
  inline val c: 1 = 1
  def f(x: Int = A.f()): Int = x
}
@main def run(): Unit = println((A.a, A.f(), B.f(), B.c))
