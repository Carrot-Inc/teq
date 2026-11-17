// A function value that stands where `Unit` is wanted and was not written there as a method's name is dropped as
// any value is: what a transparent inline call expands to (a function literal, an inline parameter handed back,
// a branch, a case, a block, a method's name inside the inline method) and what an applied function literal
// returns, at a member's body, a lambda's result, a branch and a statement.
object Notes:
  var made = 0
  def bump(i: Int): Int = { made += 1; i + 1 }

transparent inline def adder: Int => Int = (i: Int) => i + 2
transparent inline def handed(inline f: Int => Int): Int => Int = f
transparent inline def chosen(inline b: Boolean): Any = if b then (i: Int) => i + 1 else "s"
transparent inline def matched(inline k: Int): Any = inline k match { case 0 => (i: Int) => i; case _ => 1 }
transparent inline def blocked: Int => Int = { val k = 2; (i: Int) => i * k }
transparent inline def named: Int => Int = Notes.bump

def seen(x: Any): String = x match
  case () => "unit"
  case other => "a value"

class Ledger:
  def literal: Unit = adder
  def passed: Unit = handed(i => i + 1)
  def picked: Unit = chosen(true)
  def cased: Unit = matched(0)
  def ended: Unit = blocked
  def inner: Unit = named
  def given1: Unit = handed(Notes.bump)
  def reduced: Unit = ((i: Int) => (j: Int) => i + j)(1)
  val kept: Unit = adder
  lazy val later: Unit = named
  def branch(b: Boolean): Unit = if b then adder else named
  def stated(): Unit = { adder; named; handed(Notes.bump); println("after") }

def each(f: Int => Unit): Unit = println(seen(f(1)))

@main def run(): Unit =
  val l = new Ledger
  println(seen(l.literal) + " " + seen(l.passed) + " " + seen(l.picked) + " " + seen(l.cased) + " " + seen(l.ended))
  println(seen(l.inner) + " " + seen(l.given1) + " " + seen(l.reduced) + " " + seen(l.kept) + " " + seen(l.later))
  println(seen(l.branch(true)) + " " + seen(l.branch(false)))
  l.stated()
  val g: () => Unit = () => adder
  println(seen(g()))
  each(i => adder)
  each(a => ((i: Int) => (j: Int) => i + j)(a))
  val h: () => Unit = () => ((i: Int) => (j: Int) => { println("never called"); i + j })(1)
  println(seen(h()))
  each(i => handed(j => j + i))
  each(i => named)
  println(Notes.made)
