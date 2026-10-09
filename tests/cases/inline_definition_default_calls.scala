// A default of an inline method's parameter is typed at the definition with the body, and a copy of
// the stored body copies it too, with its deferred calls, its generic calls' type arguments and its
// other metadata: the census checks `f`'s default call of `id` and `g`'s of `wrap[A]`.
// Both compilers print what scalac 3.8.4 prints.
inline def id(inline x: Int): Int = x
inline def wrap[A](a: A): List[A] = List(a)
inline def f(x: Int = id(1)): Int = x
inline def g[A](a: A, xs: List[A] = wrap[A](null.asInstanceOf[A])): Int = xs.length
@main def run(): Unit =
  println(f())
  println(f(2))
  println(g("s"))
