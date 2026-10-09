// A function value made from a top-level def initialises the def's file when it is called, not
// when it is made, after the arguments of its caller: `et.f` as a value is the def itself in the
// JavaScript output, which checks the initialiser at its entry.
def apply(h: Int => Int, x: Int): Int = h(x)
@main def main(): Unit =
  val h: Int => Int = et.f
  val k: Int => Int = x => et.g(x, 1)
  println("captured")
  println(apply(h, { println("argument"); 2 }))
  println(apply(k, 3))
  val w: Int => Int = et2.withTry
  println("captured 2")
  println(w(5))
