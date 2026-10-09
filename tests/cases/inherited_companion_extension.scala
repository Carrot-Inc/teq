// An extension a companion inherits from a trait is in the implicit scope of the class and is
// called on the companion, also where no import names it (scalac: "A", "B!").
class Box(val s: String)
object Box extends BoxOps
trait BoxOps:
  extension (b: Box) def shout: String = b.s.toUpperCase
  extension (b: Box) def bang(suffix: String): String = b.s + suffix

@main def main(): Unit =
  println(Box("a").shout)
  println(Box("B").bang("!"))
