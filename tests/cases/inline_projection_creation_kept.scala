// expected: by hand
// Four shapes in which scalac 3.8.4's inliner reduces `new C(args).x` to an argument and so gives another value or
// loses an effect: a parent applied in two clauses, a parent's default argument, a case class with two parameter
// lists, a creation through a secondary constructor. teq makes the instance. The expected lines are written by
// hand: they are teq's, what the program means, and not scalac's, whose line stands beside each call.
object Test:
  val log = new StringBuilder
  def note(k: Int): Int = { log.append(k); k }
  def shown(r: Any): Unit = { println(log.toString + " " + r); log.clear() }

  class Split(val y: Int)()
  class OverSplit(val x: Int) extends Split(note(9))()
  class Preset(val y: Int = note(9))
  class OverPreset(val v: Int) extends Preset()
  class Holding(val a: OverPreset, val b: Int)
  case class Twofold(v: Int)(val w: Int)
  class Shifted(val v: Int):
    def this(v: Int, tag: String) = { this(v + 10); note(9) }
  class Passed(val v: Int):
    def this(v: Int, tag: String) = this(v)

  inline def byValue(x: Int): Int = x
  inline def ofPreset(inline m: OverPreset): Int = m.v
  inline def second(inline p: Holding): Int = p.b
  inline def ofTwofold(inline m: Twofold): Int = m.v * 10 + m.w
  inline def ofShifted(inline m: Shifted): Int = m.v
  inline def ofPassed(inline m: Passed): Int = m.v + m.v

  // The class is defined after the value: scalac's tree holds no creation of it (and the second list's argument
  // for the first parameter), so it accepts; teq accepts as long as it keeps the creation where scalac drops it.
  def later(n: Int): Int =
    def first: Int = ofLate(Late(1)(2))
    val got: Int = first
    case class Late(v: Int)(val w: Int)
    inline def ofLate(inline m: Late): Int = m.v
    got

  def main(args: Array[String]): Unit =
    val zero = args.length
    shown(byValue(new OverSplit(note(1)).x)) // scalac: "1 1", the parent's argument not evaluated
    shown(byValue(new OverPreset(note(1)).v)) // scalac: "1 1", the parent's default not evaluated
    shown(ofPreset(new OverPreset(note(1)))) // scalac: "1 1"
    shown(second(new Holding(new OverPreset(7), 5))) // scalac: " 5", the unselected creation's default not evaluated
    shown(ofTwofold(Twofold(zero + 1)(zero + 2))) // scalac: " 22", the second list's argument for `v`
    shown(later(3)) // scalac: " 2"
    shown(ofShifted(new Shifted(note(1), "t"))) // scalac: "1 1", the constructor's body not run, its argument for `v`
    shown(ofPassed(new Passed(note(1), "t"))) // scalac: "11 2", as teq: the constructor passes its argument on
