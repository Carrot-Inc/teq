// A tuple's element whose prototype is or holds a prototype wildcard (a default's `wildApprox`) takes the element's
// own type, adapted to the prototype; a written wildcard element (`? <: Long`, `? >: String`) bounds it as written.
object R:
  def kind(x: Any): String = x match
    case _: java.lang.Long => "long"
    case _: java.lang.Integer => "int"
    case _ => "other"
  def pair[T](p: (T, T) = (1, 2)): (T, T) = p
  def option[T](p: (Int, Option[T]) = (1, None)): (Int, Option[T]) = p
  def nested[T](p: ((T, Int), Int) = ((1, 2), 3)): ((T, Int), Int) = p
  def named[T](p: (a: T, b: Int) = (a = 1, b = 2)): (a: T, b: Int) = p
  def lower[T >: String](p: (T, Int) = ("s", 2)): (T, Int) = p
  def upper[T <: Long](p: (T, T) = (1, 2)): (T, T) = p
@main def run(): Unit =
  val both: (Int, Int) = R.pair()
  println(both)
  println(R.option())
  println(R.nested())
  println(R.named())
  println(R.lower())
  println(R.kind(R.upper()._1))
  val bounded: (? <: Long, Int) = (1, 2)
  println(R.kind(bounded._1))
  val below: (? >: String, Int) = (1, 2)
  println(below)
  val written: (Option[?], Int) = (Some(1), 2)
  println(written)
  val f: Function1[Int, ? <: Long] = x => 1
  println(R.kind(f(3)))
