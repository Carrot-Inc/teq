// A case class's companion declaring an `unapply` of its own beside the synthetic one: a
// scrutinee that is no instance of the class is matched through it, as overload resolution
// on the scrutinee picks it.
final case class Ctor[To](params: List[String], make: String => To)
object Ctor:
  def unapply[To](t: Option[To]): Option[(List[String], String => Option[To])] =
    t.map(v => (List("p"), (s: String) => Some(v)))

def parse[A](t: Option[A]): String = t match
  case Ctor(params, make) => params.mkString + make("x")
  case _ => "none"

@main def main(): Unit =
  println(parse(Some(3)))
  println(parse(None))
  println(Ctor(Nil, _.length) match { case Ctor(ps, f) => f("abc") })
