// A member whose declared type is `Unit` evaluates its body's value once and drops it, whatever the member is
// and wherever the value stands in the body (a branch, a case, a handler, a block's end); a context function
// is applied before its result is dropped; an expected `Unit`, through an alias too, does not decide what a
// generic call's argument is.
object Tally:
  var count = 0
  def bump(): Int = { count += 1; count }

type Quiet = Unit
given step: Int = 7
def stepped: Int ?=> Int = { Tally.bump(); summon[Int] }
def same[A](a: A): A = { println("same " + a); a }
def seen(x: Any): String = x match
  case () => "unit"
  case other => "a value: " + other

class Ledger:
  val entered: Unit = Tally.bump()
  var slot: Unit = Tally.bump()
  lazy val later: Unit = Tally.bump()
  def paid: Unit = Tally.bump()
  def chosen(b: Boolean): Unit = if b then Tally.bump() else Tally.bump() + 1
  def matched(k: Int): Unit = k match { case 0 => Tally.bump(); case _ => Tally.bump().toString }
  def tried: Unit = try Tally.bump() catch { case _: RuntimeException => Tally.bump() }
  def ended: Unit = { Tally.bump(); Tally.bump() }
  def aliased: Quiet = same(2)
  def applied: Unit = stepped
  def inside: Unit = { def inner: Unit = Tally.bump(); println(seen(inner)); Tally.bump() }

abstract class Shape:
  def draw(): Unit

class Dot extends Shape:
  def draw(): Unit = Tally.bump()

object Holder:
  given marker: Unit = Tally.bump()

val top: Unit = Tally.bump()

@main def run(): Unit =
  val l = new Ledger
  println(seen(l.entered) + " " + seen(l.slot) + " " + seen(l.later) + " " + seen(l.later) + " " + seen(l.paid))
  println(seen(l.chosen(true)) + " " + seen(l.matched(1)) + " " + seen(l.tried) + " " + seen(l.ended))
  println(seen(l.aliased) + " " + seen(l.applied))
  println(seen(l.inside))
  val shape: Shape = new Dot
  println(seen(shape.draw()) + " " + seen(Holder.marker) + " " + seen(Holder.marker) + " " + seen(top))
  println(Tally.count)
