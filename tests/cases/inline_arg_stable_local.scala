// A plain `match` over an inline parameter types its patterns against the parameter's declared
// type, as scalac typed them at the definition, whatever the argument's own type: a local or a
// path bound to `Nil` or `None`, as an object argument itself.
inline def head(xs: List[Int]): Int = xs match
  case h :: _ => h
  case Nil => 0
inline def orZero(o: Option[Int]): Int = o match
  case Some(n) => n
  case None => 0
object Holder:
  val none: Option[Int] = None
  val empty = Nil
@main def run(): Unit =
  val xs = Nil
  println(head(xs))
  println(head(List(4, 5)))
  println(head(Nil))
  println(orZero(None))
  println(orZero(Holder.none))
  println(head(Holder.empty))
  val some = Some(3)
  println(orZero(some))
