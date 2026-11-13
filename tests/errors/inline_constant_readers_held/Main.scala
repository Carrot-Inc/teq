// The refused cells of tests/errors/inline_constant_readers.scala in a body the definition's check
// holds back (tests/cases/inline_constant_readers_held).
// expect: 30:13: error: Cannot reduce `inline if` because its condition is not a constant value: b.asInstanceOf[Boolean]
// expect: 31:13: error: Cannot reduce `inline if` because its condition is not a constant value: !b.asInstanceOf[Boolean]
// expect: 32:13: error: cannot reduce inline match with
import scala.quoted.*

object Lib:
  inline def yes: Boolean = true

object Gate:
  transparent inline def inlineIf_cast(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      inline if b.asInstanceOf[Boolean] then new Cell(100).v else 200
    }) }
  transparent inline def operand_cast(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      inline if !b.asInstanceOf[Boolean] then 200 else new Cell(100).v
    }) }
  transparent inline def guard_cast(inline b: Boolean): String =
    ${ Quoted.same('{
      class Cell(val v: String)
      inline 1 match
        case _ if b.asInstanceOf[Boolean] => new Cell("nine").v
        case _ => "other"
    }) }

val cell0 = Gate.inlineIf_cast((true: Boolean)) // inlineIf, under a cast to Boolean
val cell1 = Gate.operand_cast((true: Boolean)) // operand, under a cast to Boolean
val cell2 = Gate.guard_cast((true: Boolean)) // guard, under a cast to Boolean
