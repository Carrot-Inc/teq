//> using options -nowarn
object Arguments:
  def keep(f: Any => Any): Any => Any = f
  val f = keep({ case (x, y) => x }: Any => Any)
  def consume(values: Any*): Int = values.size
  def sum: Int = consume({ val xs = Array[Any](1, 2); xs }: _*)
@main def blockArguments(): Unit =
  println(Arguments.f((3, 4)))
  println(Arguments.sum)
