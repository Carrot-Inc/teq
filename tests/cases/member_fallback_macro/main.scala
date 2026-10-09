import Counter.next

class C { def f(a: String): String = "member " + a }
extension (c: C) def f(a: String, b: Int): String = "extension " + a + b

@main def run(): Unit =
  println(C().f(next, 0))
  println(C().f(next))
  println(next)
