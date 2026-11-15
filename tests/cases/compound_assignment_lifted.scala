// `lhs op= rhs` where `lhs` has no `op=` is `lhs = lhs op rhs` with the left side evaluated once: what it is
// selected on, and what an indexed form is applied to, is bound in front unless it is a path that evaluates
// nothing; an argument of a by-name or an inline parameter is not bound. An `inline` parameter read there is
// bound where the method is defined, so its argument is evaluated once at each call.
object Test:
  val log = new StringBuilder
  def note(k: Int): Int = { log.append(k); k }
  def mark(s: String): Int = { log.append(s); 1 }
  def line(): String = { val s = log.toString; log.clear(); s }

  class Slot(var v: Int)
  class Rack(val slot: Slot)
  class Meter:
    private var held: Int = 10
    def level: Int = { log.append("g"); held }
    def level_=(v: Int): Unit = { log.append("s"); held = v }
  class Shelf:
    def apply(i: => Int): Int = { log.append("G"); i; 10 }
    def update(i: => Int, v: Int): Unit = { log.append("S"); i; () }
  class Tally:
    var count: Int = 0
    def +=(k: Int): Tally = { log.append("A"); count += k; this }
  class Ledger:
    val tally = new Tally
    def apply(i: Int): Tally = { log.append("G"); tally }
  object Store:
    log.append("O")
    var count: Int = 10
    def apply(i: Int): Int = { log.append("G"); count }
    def update(i: Int, x: Int): Unit = { log.append("S"); count = x }

  inline def raise(inline s: Slot): Unit = s.v += 1
  transparent inline def raiseNow(inline s: Slot): Unit = s.v += 1
  inline def raiseTwice(inline s: Slot): Unit = { s.v += 1; s.v *= 2 }
  inline def raiseAt(inline xs: Array[Int], inline i: Int): Unit = xs(i) += 1
  inline def lift(inline m: Meter): Unit = m.level += 1
  inline def deferred(inline g: Shelf, inline i: Int): Unit = g(i) += mark("R")

  def made(): Slot = { note(1); new Slot(1) }
  def cells(): Array[Int] = { note(1); Array(1, 2) }
  def at(): Int = { note(2); 0 }
  def meter(): Meter = { note(1); new Meter }

  def main(args: Array[String]): Unit =
    raise(new Slot(note(1)))
    println("an inline parameter selected on: " + line())
    raiseNow(new Slot(note(1)))
    println("in a transparent method: " + line())
    raiseTwice(new Slot(note(1)))
    println("two assignments in one body: " + line())
    raiseAt(cells(), at())
    println("an indexed form over inline parameters: " + line())
    lift(meter())
    println("a getter and setter pair: " + line())
    Store(mark("I")) += mark("R")
    println("an object with an initializer applied: " + line() + " " + Store.count)
    val shelf = new Shelf
    deferred(shelf, mark("I"))
    println("an index of a by-name parameter: " + line())
    val ledger = new Ledger
    ledger(mark("I")) += mark("R")
    println("an operator that is a member: " + line() + " " + ledger.tally.count)
    made().v += 1
    cells()(at()) += 1
    val rack = new Rack(new Slot(3))
    rack.slot.v += 1
    println("a call, an array's element, a path: " + line() + " " + rack.slot.v)
