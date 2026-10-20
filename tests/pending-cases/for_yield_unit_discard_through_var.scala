// Pinned for the queued part inlining-lila-folds: the commit that made this case pass (db0a8ede5, the lila folds'
// eta-expansion and context-function changes) broke tests/scala3's boehm-berarducci (scala3-run and scala3-interp)
// and pos/i12211, pos/i3945 against master (the gate of 5010f41f8), a method value eta-expanded against an expected
// function type with its variables solved to Any; reverted on 2026-10-20. scalac accepts and prints the expected
// output.
// A `for` over a `map` whose result variable is bounded by another call's variable, itself
// bounded by `Unit`: the body's value is discarded, as dotty instantiates the variable from its
// full upper bound (`ConstraintHandling.fullUpperBound`) and adapts the body again to `Unit`.
class Box[+A](val value: A):
  def flatMap[B](f: A => Box[B]): Box[B] = f(value)
  def map[B](f: A => B): Box[B] = Box(f(value))
object Box:
  def apply[A](a: A): Box[A] = new Box(a)
  val unit: Box[Unit] = Box(())
type Bunit = Box[Unit]
class Counter:
  var n = 0
  def increment(): Counter = { n += 1; this }
object mon:
  val mark = new Counter
def act(): Bunit = Box.unit
def getTitle(): Box[Option[String]] = Box(None)
def markOrReport(cond: Boolean): Bunit =
  getTitle().flatMap { title =>
    if cond && title.isEmpty then
      for _ <- act()
      yield mon.mark.increment()
    else if cond then
      for
        s <- Box("s")
        _ <- act()
      yield mon.mark.increment()
    else Box.unit
  }
object Main:
  def main(args: Array[String]): Unit =
    markOrReport(true)
    markOrReport(false)
    println(mon.mark.n)
