// Explicit type arguments keep the overloaded alternatives with that many type parameters, as
// scalac narrows the set: `Array[Byte](71, 73)` is scala-library's generic `apply`, whose
// parameter type then narrows the literals; a program's overloads narrow the same way.
object Pick:
  def of(x: Int): String = s"int $x"
  def of[T](x: T): String = s"generic $x"

@main def run(): Unit =
  val a = Array[Byte](71, 73, 70)
  val b = Array[Short](1, 2)
  val c = Array[Long](1, 2)
  val d = Array[Double](1, 2)
  println(a.mkString(",") + " " + b.length + " " + c.sum + " " + d.length)
  println(Pick.of(1))
  println(Pick.of[Int](1))
