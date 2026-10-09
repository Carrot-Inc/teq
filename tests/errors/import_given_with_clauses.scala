// expect: 7:10: error: C is not a valid import prefix, since it is not an immutable path
// A given with a using clause is a method, not a stable path an import may select on.
class C:
  val n = 1
given c(using Int): C = new C
def f(using Int) =
  import c.*
  n
@main def run(): Unit =
  given Int = 0
  println(f)
