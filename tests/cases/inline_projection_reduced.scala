// A creation of a class that initialises nothing, selected for one of its parameters where an inline method
// expands, is that argument, as scalac's inliner reduces it: where it is the whole right side of a parameter's or
// a receiver's binding, and where the method's body selects on a creation of its own or on an `inline`
// parameter's argument. The creation's other arguments keep their effects and their order, and a class defined
// later in the block may be created so: the tree holds no creation where forward references are checked.
object Test:
  val log = new StringBuilder
  def note(k: Int): Int = { log.append(k); k }
  def shown(r: Any): Unit = { println(log.toString + " " + r); log.clear() }

  class Cell(val v: Int)
  class Trio(val a: Int, val b: Int, val c: Int)
  class Slot(var v: Int)
  case class Wrap[T](v: T)
  class Outer(val cell: Cell, val k: Int)
  trait Sized { def v: Int }
  class Leaf(val v: Int) extends Sized
  final class Closed(private val v: Int):
    inline def other(inline o: Closed): Int = o.v + 1
  class Ranked(val v: Int, val w: Int = note(9))
  class Counter(val w: Int):
    inline def more: Int = w + w
  class Holder(val counter: Counter, val k: Int)

  inline def byValue(x: Int): Int = x + x
  inline def inlined(inline x: Int): Int = x + x
  inline def bodyOwn(k: Int): Int = new Cell(k).v + 1
  inline def onParam(inline c: Cell): Int = c.v + c.v
  inline def second(inline t: Trio): Int = t.b + t.c
  inline def through(inline o: Outer): Int = o.cell.v
  inline def wrapped(inline w: Wrap[Cell]): Int = w.v.v
  inline def made(k: Int): Cell = new Cell(k)
  inline def ofCall(inline c: Cell): Int = c.v + c.v
  inline def ofBodyCall(k: Int): Int = made(note(k)).v + made(note(k + 1)).v
  inline def asParent(inline s: Sized): Int = s.v + 1
  inline def varied(k: Int): Int = new Slot(k).v + 1
  inline def ranked(inline r: Ranked): Int = r.v
  inline def choose(inline c: Cell): Int = inline if c.v > 0 then 1 else 2
  transparent inline def exact(inline c: Cell): Int = c.v
  inline def named(inline c: Cell): String = inline c.v match
    case 1 => "one"
    case _ => "other"
  inline def defaulted(inline x: Int = new Trio(note(1), note(2), 3).a): Int = x + x
  inline def blocked(k: Int): Int = { val j = 4; new Cell(j) }.v + k
  inline def applied(inline w: Wrap[Int => Int]): Int = w.v(3)

  def later(n: Int): Int =
    def first: Int = byValue(new Mid(n).v) + bodyLater(n) + onLater(new Mid(n)) + new Pair(new Counter(n), 1).counter.more
    val got: Int = first
    class Mid(val v: Int)
    class Pair(val counter: Counter, val k: Int)
    inline def bodyLater(k: Int): Int = new Mid(k).v
    inline def onLater(inline m: Mid): Int = m.v
    got

  def main(args: Array[String]): Unit =
    val zero = args.length
    shown(byValue(new Cell(note(1)).v))
    shown(byValue(new Trio(note(1), note(2), note(3)).b))
    shown(byValue(new Trio(note(1), note(2), note(3)).c))
    shown(inlined(new Trio(note(1), note(2), note(3)).a))
    shown(byValue({ val j = 5; new Cell(j).v }))
    shown(byValue(new Wrap(new Cell(note(1)).v).v))
    shown(new Holder(new Counter(note(1)), note(2)).counter.more)
    shown(bodyOwn(note(4)))
    shown(onParam(new Cell(note(1))))
    shown(second(new Trio(note(1), note(2), note(3))))
    shown(through(new Outer(new Cell(note(1)), note(2))))
    shown(wrapped(Wrap(new Cell(note(1)))))
    shown(ofCall(made(note(1))))
    shown(ofBodyCall(1))
    shown(asParent(new Leaf(note(1))))
    shown(varied(note(6)))
    shown(new Closed(0).other(new Closed(note(1))))
    shown(ranked(new Ranked(note(1))))
    shown(choose(new Cell(3)))
    val one: 1 = exact(new Cell(1))
    shown(one)
    shown(named(new Cell(1)))
    shown(defaulted())
    shown(blocked(zero + 1))
    shown(applied(Wrap((x: Int) => x + zero + 1)))
    shown(try second(new Trio(1, 7 / zero, 5)) catch { case _: ArithmeticException => -1 })
    shown(later(3))
