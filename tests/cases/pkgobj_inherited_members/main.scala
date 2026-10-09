// The members a package object inherits from its parents are members of the package: exported
// by name, imported by name and by a wildcard (scalac: "a 1 b 3 event c").
object Prelude:
  export events.{ReactEvent, ReactKeyboardEvent, describe}
object Named:
  import events.ReactEvent
  def f(e: ReactEvent): String = e
object Wild:
  import events.*
  def g(k: ReactKeyboardEvent): Int = k + 1
import Prelude.*
@main def main(): Unit =
  val e: ReactEvent = "a"
  val k: events.ReactKeyboardEvent = 1
  println(s"$e $k ${Named.f("b")} ${Wild.g(2)} ${describe("c")}")
