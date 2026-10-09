// teq: --werror
// A literal sub-pattern against the singleton type an extractor gives it cannot fail, so the
// pattern val is not warned about, as scalac has it.
object E:
  def unapply(x: Int): Some[1] = Some(1)
@main def run(): Unit =
  val E(1) = 2
  println("ok")
