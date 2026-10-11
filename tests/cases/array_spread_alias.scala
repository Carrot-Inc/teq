// An array spread into a Scala varargs parameter is wrapped as it is, not copied (dotty's `ElimRepeated.arrayToSeq`,
// `ScalaRunTime.wrapRefArray` and its primitive and generic siblings), so a later write to the array shows in the
// sequence (master's JVM printed `before` and `ArraySeq(1, 2)`: the typer's conversion copied it); a sequence spread
// stays itself.
def keep(xs: String*): Seq[String] = xs
def ints(xs: Int*): Seq[Int] = xs
def gen[T](xs: T*): Seq[T] = xs
def anys(xs: Any*): Seq[Any] = xs
def units(xs: Unit*): Seq[Unit] = xs
def spreadGen[T](a: Array[T]): Seq[T] = gen(a*)
@main def run() =
  val a = Array("before")
  val xs = keep(a*)
  a(0) = "after"
  println(xs.head)
  val i = Array(1, 2)
  val is = ints(i*)
  i(0) = 9
  println(is)
  val g = Array(1.5)
  val gs = spreadGen(g)
  g(0) = 2.5
  println(gs)
  val o: Array[Any] = Array(1, "x")
  val os = anys(o*)
  o(0) = 3
  println(os)
  val ys = keep(Seq("s")*)
  println(ys)
  println(units(Array((), ())*).length)
