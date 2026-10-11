// A polymorphic function type whose parameter is bounded by the enclosing method's type parameter:
// the substitution of that parameter reaches the bound and the body, the function type's own
// parameter staying bound (dotty's `TypeMap.mapOverLambda`).
def f[U](u: U)(g: [T <: U] => T => U): U = g[U](u)

@main def run(): Unit =
  val r: Int = f[Int](3)([T <: Int] => (t: T) => t + 1)
  println(r)
  val s: String = f("a")([T <: String] => (t: T) => t + "b")
  println(s)
