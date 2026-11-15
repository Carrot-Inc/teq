// Where scalac's inliner keeps a creation that is selected for a parameter, so does teq: the creation is made
// once and its class initialised. A selection inside an argument, a by-name parameter's argument, an ascribed
// one, a by-value parameter bound to the creation itself, a value of the method's body; a class with a second
// parameter list, with a value or a statement in its body, with a parent that is applied to an argument; a
// parameter the method reads through an accessor that takes the receiver or has an expanded name.
object Test:
  val log = new StringBuilder
  def note(k: Int): Int = { log.append(k); k }
  def shown(r: Any): Unit = { println(log.toString + " " + r); log.clear() }

  class Cell(val v: Int)
  class Twice(val v: Int)(val w: Int)
  class Summed(val v: Int) { val sum: Int = v + note(7) }
  class Loud(val v: Int) { note(8) }
  class Base(val k: Int)
  class Derived(val v: Int) extends Base(note(6))
  class Outer(val cell: Cell, val k: Int)
  class Hidden(private val v: Int)
  object Hidden:
    inline def read(inline h: Hidden): Int = h.v + 1
  class Open(private val v: Int):
    inline def other(inline o: Open): Int = o.v + 1

  inline def byValue(x: Int): Int = x + x
  inline def inlined(inline x: Int): Int = x + x
  inline def byName(c: => Cell): Int = c.v + c.v
  inline def bound(c: Cell): Int = c.v + c.v
  inline def ofBody(k: Int): Int = { val c = new Cell(note(k)); c.v + c.v }
  inline def twice(inline t: Twice): Int = t.v + t.w
  inline def summed(inline s: Summed): Int = s.v
  inline def loud(inline l: Loud): Int = l.v
  inline def derived(inline d: Derived): Int = d.v
  inline def ordinary(x: Int = new Cell(note(1)).v): Int = x + x
  inline def applied(f: Int => Int): Int = f(1) + f(2)

  def main(args: Array[String]): Unit =
    shown(byValue(new Cell(note(1)).v + 1))
    shown(inlined(new Cell(note(1)).v + 1))
    shown(byValue((new Cell(note(1)).v): Int))
    shown(byValue(new Outer(new Cell(note(1)), note(2)).cell.v))
    shown(applied(x => new Cell(note(x)).v))
    shown(byName(new Cell(note(1))))
    shown(bound(new Cell(note(1))))
    shown(ofBody(1))
    shown(twice(new Twice(note(1))(note(2))))
    shown(summed(new Summed(note(1))))
    shown(loud(new Loud(note(1))))
    shown(derived(new Derived(note(1))))
    shown(ordinary())
    shown(Hidden.read(new Hidden(note(1))))
    shown(new Open(0).other(new Open(note(1))))
