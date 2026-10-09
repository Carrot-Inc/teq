// A case class's companion with a body is initialised before the construction's arguments run, as
// scalac's `apply` is called on it, and once.
final case class Flags(on: Boolean, names: List[String])
object Flags:
  println("Flags initialised")
  val empty: Flags = Flags(false, Nil)

@main def run(): Unit =
  val a = Flags(on = true, names = { println("names"); List("a") })
  val b = Flags(false, { println("again"); Nil })
  println(a)
  println(b == Flags.empty)
