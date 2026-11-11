// An inline parameter's default that reads a private member reads it through its class's inline accessor, as the
// getter is an inline method of its own (Desugar.scala:429) whose body makeInlineable prepares: so a build from the
// products, whose expansion is outside the class, reads it. scalac prints the lines of the .expected file.
class Strict(k: Int):
  inline def f(inline x: Int = { println("sx"); k })(y: Int = x * 2): Int = y
class R(private val k: Int):
  inline def f(inline x: Int = k + 1): Int = x
@main def run(): Unit =
  println(Strict(6).f()())
  println(R(3).f())
  println(Strict(5).f(1)())
