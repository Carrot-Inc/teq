// Two imports of one type under its name, an export's type forwarder and the original, bind it once:
// no ambiguity, as dotty's `isSameRef` finds each a subtype of the other (a forwarder of an alias that
// takes parameters, `[A] =>> lib.Box[A]`, and of a class).
package app

object lib:
  type Box[A] = List[A]
  class Tag(val s: String)

object Prelude:
  export lib.{Box, Tag}

object Use:
  import Prelude.*
  import lib.*
  def boxed: Box[Int] = List(1, 2)
  def tag: Tag = new Tag("t")

@main def run(): Unit =
  println(Use.boxed)
  println(Use.tag.s)
