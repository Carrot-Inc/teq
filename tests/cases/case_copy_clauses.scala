// A case class's `copy` is dotty's synthesized method (`Desugar.copyMeths`): the first clause's
// parameters default to the receiver's fields, the later clauses have no defaults and are
// applied as any method's, a using clause resolved where the copy is written; a missing list
// eta-expands, and the arguments are evaluated as written.
case class C(a: Int)(val b: Int)
case class G(a: Int)(using val s: String)
case class T[A](a: A)(val b: A)
case class U(a: Int, s: String)(val b: Int)(using val g: String)

@main def run(): Unit =
  val c = C(0)(9)
  val d = c.copy(a = 1)(2)
  println((d.a, d.b))
  val e = c.copy()({ println("second"); 3 })
  println((e.a, e.b))
  val f = c.copy(a = 5)
  println(f(6).b)
  val all = c.copy
  println(all(4)(5).b)
  val h: Int => C = c.copy(a = 7)
  println(h(8).b)
  val t = T(1)(2)
  val u = t.copy(a = "x")("y")
  println((u.a, u.b))
  val v = t.copy()("y")
  println((v.a, v.b))
  given String = "site"
  val g = G(1)(using "original")
  println(g.copy(a = 2).s)
  val x = U(1, "s")(2)(using "original")
  val y = x.copy(s = { println("s"); "t" }, a = { println("a"); 3 })({ println("b"); 4 })
  println((y.a, y.s, y.b, y.g))
