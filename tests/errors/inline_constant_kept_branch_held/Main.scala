// The plain `if` of tests/errors/inline_constant_kept_branch.scala in a body the definition's
// check holds back: scalac reports it where the other branch is written, in the quote.
// expect: 16:13: error: the other branch is kept
import scala.quoted.*

object Lib:
  inline def yes: Boolean = true

object Gate:
  transparent inline def plainIf_cast(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      if b.asInstanceOf[Boolean] then new Cell(100).v else scala.compiletime.error("the other branch is kept")
    }) }

val cell0 = Gate.plainIf_cast((true: Boolean)) // plainIf, under a cast to Boolean
