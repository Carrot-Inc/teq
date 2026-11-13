// A condition under a cast selects its branch by the cast's type and is still evaluated before
// it, once: an argument's effect happens, and a cast that fails throws. In a body the method's
// walk copies and in one the definition's check holds back (a class made in a quote), in an
// `inline if` and in a plain `if`; a method a line where the body is held back.
import scala.quoted.*

object Effects:
  def yes(): Boolean = { println("evaluated"); true }
  def no(): Boolean = { println("evaluated"); false }

object Gate:
  transparent inline def walked(inline b: Boolean): Int =
    inline if b.asInstanceOf[true] then 100 else 200
  transparent inline def walkedPlain(inline b: Boolean): Int =
    if b.asInstanceOf[true] then 100 else 200
  transparent inline def walkedFailing(inline a: Any): Int =
    inline if a.asInstanceOf[true] then 100 else 200
  transparent inline def heldYes(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      inline if b.asInstanceOf[true] then new Cell(100).v else 200
    }) }
  transparent inline def heldNo(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      inline if b.asInstanceOf[true] then new Cell(100).v else 200
    }) }
  transparent inline def heldBlock(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      inline if b.asInstanceOf[true] then new Cell(100).v else 200
    }) }
  transparent inline def heldPlainYes(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      if b.asInstanceOf[true] then new Cell(100).v else 200
    }) }
  transparent inline def heldPlainNo(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      if b.asInstanceOf[true] then new Cell(100).v else 200
    }) }
  transparent inline def heldFailing(inline a: Any): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      inline if a.asInstanceOf[true] then new Cell(100).v else 200
    }) }
  transparent inline def heldPlainFailing(inline a: Any): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      if a.asInstanceOf[true] then new Cell(100).v else 200
    }) }

def failing(body: => Int): String =
  try body.toString catch case _: ClassCastException => "the cast failed"

@main def run(): Unit =
  println("walked, a call that answers true: " + Gate.walked(Effects.yes()))
  println("walked, a call that answers false: " + Gate.walked(Effects.no()))
  println("walked, a plain if: " + Gate.walkedPlain(Effects.no()))
  println("walked, a cast that fails: " + failing(Gate.walkedFailing("text")))
  println("held back, a call that answers true: " + Gate.heldYes(Effects.yes()))
  println("held back, a call that answers false: " + Gate.heldNo(Effects.no()))
  println("held back, a block: " + Gate.heldBlock({ println("evaluated"); true }))
  println("held back, a plain if, true: " + Gate.heldPlainYes(Effects.yes()))
  println("held back, a plain if, false: " + Gate.heldPlainNo(Effects.no()))
  println("held back, a cast that fails: " + failing(Gate.heldFailing("text")))
  println("held back, a plain if, a cast that fails: " + failing(Gate.heldPlainFailing("text")))
