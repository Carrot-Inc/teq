// Named arguments out of parameter order, as dotty's `Application` lifts them (Applications.scala
// 1219): only where not every argument is safe (a pure expression, a default's value), in the
// order written; `LiftComplex` for a method with default parameters leaves a pure path in place,
// `LiftImpure` for one without leaves a pure expression; a case class's copy reads the receiver's
// fields for the parameters left out.
final case class Draft(id: Int, description: String, image: Option[String], effect: Int)

var log = List.empty[String]
def note[A](s: String, a: A): A =
  log = s :: log
  a
def flush(): Unit =
  println(log.reverse.mkString(" "))
  log = Nil

def plain(a: Int, b: Int, c: String): String = s"$a $b $c"
def withDefault(a: Int, b: Int = 9, c: String = "z"): String = s"$a $b $c"

@main def run(): Unit =
  val d = Draft(1, "first", None, 2)
  println(d.copy(image = Some("x")))
  println(d.copy(effect = note("effect", 3), description = note("description", "second")))
  flush()
  println(d.copy(effect = note("effect", 4), id = d.id))
  flush()
  println(plain(c = note("c", "w"), a = note("a", 1), b = 2))
  flush()
  println(plain(c = "v", b = d.id, a = note("a", 5)))
  flush()
  println(withDefault(c = note("c", "y"), a = note("a", 4)))
  flush()
