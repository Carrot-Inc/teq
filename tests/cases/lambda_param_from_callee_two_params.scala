// Two parameters of a lambda, each left open by the expected type and each an argument of the callee once.
final case class Tag(name: String)
def both(name: String, n: Int): Tag = Tag(name * n)
def on2[B, C, R](f: (B, C) => R): (B, C) => R = f

@main def run(): Unit =
  val named = on2((s, n) => both(s, n))
  val swapped = on2((n, s) => both(s, n))
  val placeholders = on2(both(_, _))
  println(named("a", 2).name + " " + swapped(3, "b").name + " " + placeholders("c", 1).name)
