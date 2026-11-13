// Paths with no export on them keep their values and what they initialise: an inline val read through `this`
// and through a val, a literal lazy val (initialised by its `ValueOf`), a constant val of an object with an
// initialiser and of an object nested in an instance (neither initialised).
package paths

class Holder:
  inline val width = 3
  def within: Int = valueOf[width.type]

object Lazily:
  lazy val literal: 7 = { println("literal-init"); 7 }

class Outer:
  object Inner:
    println("inner-init")
    final val x: 7 = 7

object Still:
  println("still-init")
  final val y: 8 = 8

@main def run(): Unit =
  val h = Holder()
  println(s"${h.within} ${valueOf[h.width.type]}")
  println(valueOf[Lazily.literal.type])
  println("between")
  println(Lazily.literal)
  val o = new Outer
  println(valueOf[o.Inner.x.type])
  println(valueOf[Still.y.type])
  println(Still.y)
