// expect: 7:25: error: Cannot prove that Int <:< Option[B].
// expect: 8:33: error: Cannot prove that String <:< Int.
// expect: 9:30: error: Cannot prove that Int =:= Any.
final case class Box[A](value: A):
  def flatten[B](using ev: A <:< Option[B]): Option[B] = ev(value)
@main def main(): Unit =
  println(Box(3).flatten)
  println(summon[String <:< Int])
  println(summon[Int =:= Any])
