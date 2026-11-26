// A right-associative extension with interleaved type clauses, called with each type argument where its clause stands.
object Lib:
  extension [A](using b: Ordering[A])(using d: DummyImplicit)(x: A)
    def %:[E](y: E)(z: E): String = x.toString + y.toString + z.toString
@main def run(): Unit =
  println(Lib.%:[Int](using summon[Ordering[Int]])(using summon[DummyImplicit])[String]("y")(1)("z"))
