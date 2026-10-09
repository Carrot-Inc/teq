// `new A(args)` of a parameterised alias with no type arguments constructs the aliased class with
// all of its arguments inferred, those the alias fixes included.
final class Cell2[F, X](val x: X, val f: F)
type C[X] = Cell2[String, X]

@main def run(): Unit =
  val c: Cell2[Int, Int] = new C(3, 5)
  val d = new C(3, 5)
  val e: C[Int] = new C(4, "s")
  println(c.f + d.f + e.x)
  println(e.f.length)
