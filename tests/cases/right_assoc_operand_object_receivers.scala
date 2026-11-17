// A right-associative operator's application `l op: r` is `r.op:(l)` with `l` evaluated first: the left operand
// is bound ahead of the receiver unless the receiver's evaluation can do nothing. Every row has objects of its
// own; an object's loading shows through a parent trait's parameter default (its own class writes no
// initialisation) unless the row says otherwise.
def effect(tag: String): Int = { println("  effect " + tag); 1 }

trait Shown(tag: String)(x: Int = { println("  " + tag + " loaded"); 1 })

object Kept1 extends Shown("Kept1")() { def ::(x: Any): Unit = println("  :: called") }
object Loud2 { println("  Loud2 loaded"); def ::(x: Any): Unit = println("  :: called") }
object Quiet3 { def ::(x: Any): Unit = println("  :: called") }
class Cell { def +:(x: Any): Unit = println("  +: called") }
object Holder4 extends Shown("Holder4")() { val cell = new Cell }
def made(tag: String): Cell = { println("  made " + tag); new Cell }
object Kept6 extends Shown("Kept6")() { def ::(x: Any): Unit = println("  :: called") }
object Kept7 extends Shown("Kept7")() { def ::(x: Any): Kept7.type = { println("  :: called with " + x); this } }
object Kept8 extends Shown("Kept8")()
extension (x: Int) def +::(k: Kept8.type): Unit = println("  +:: called")
object Elem9 extends Shown("Elem9")()
object Recv9 extends Shown("Recv9")() { def ::(xs: Any*): Unit = println("  :: called with " + xs.length) }
object Kept10 extends Shown("Kept10")() { def ::(x: Any): Unit = println("  :: called") }
inline def consed(inline r: Kept10.type): Unit = effect("l") :: r
object Kept11 extends Shown("Kept11")() { def ::(x: Any): Unit = println("  :: called") }
object Arg12 extends Shown("Arg12")()
object Kept12 extends Shown("Kept12")() { def ::(x: Any): Unit = println("  :: called") }
object Kept13 extends Shown("Kept13")() { def ::(x: => Any): Unit = { println("  :: entered"); x; println("  :: left") } }

@main def run(): Unit =
  println("01 an object whose load shows, the flag set"); effect("l") :: Kept1
  println("02 an object with a statement"); effect("l") :: Loud2
  println("03 an object that runs nothing"); effect("l") :: Quiet3
  println("04 a value of an object whose load shows"); effect("l") +: Holder4.cell
  println("05 a call as the right operand"); effect("l") +: made("r")
  println("06 a literal on the left"); 5 :: Kept6
  println("07 a chain"); effect("a") :: effect("b") :: Kept7
  println("08 an extension operator"); effect("l") +:: Kept8
  println("09 a repeated parameter: an effect and an object"); (effect("a"), Elem9) :: Recv9
  println("10 in an inline method's body, the object its argument"); consed(Kept10)
  println("11 a block on the left"); { println("  block"); 2 } :: Kept11
  println("12 an object on the left"); Arg12 :: Kept12
  println("13 a by-name parameter"); effect("l") :: Kept13
  println("end")
