// expect: no given instance of type <:<[Int, Option[B]] was found
// expect: no given instance of type <:<[String, Int] was found
// expect: no given instance of type =:=[Int, Any] was found
final case class Box[A](value: A):
  def flatten[B](using ev: A <:< Option[B]): Option[B] = ev(value)
@main def main(): Unit =
  println(Box(3).flatten)
  println(summon[String <:< Int])
  println(summon[Int =:= Any])
