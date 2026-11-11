import scala.quoted.*

object QuotedSum:
  def add(left: Expr[Int], right: Expr[Int])(using Quotes): Expr[Int] =
    '{ $left + $right }
  def nested(left: Expr[Int], right: Expr[Int])(using Quotes): Expr[Int] =
    '{ ${ add(left, right) } + 1 }
  def identity(value: Expr[Int])(using Quotes): Expr[Int] = value

// Each quoted reference keeps its owner across inline copying and splicing.
// The same stored method can be expanded on different enclosing instances.
class Enclosing(private val value: Int):
  class Inner(val own: Int):
    inline def add(inline n: Int): Int = ${ QuotedSum.add('{ value }, 'n) }
    inline def both: Int = ${ QuotedSum.add('{ own }, '{ value }) }
    inline def nested(inline n: Int): Int =
      ${ QuotedSum.nested('{ value }, 'n) }
    inline def local(inline n: Int): Int = ${ QuotedSum.identity('{
      class Cell(val own: Int):
        def sum: Int = own + value
      new Cell(n).sum
    }) }
    class Deeper(val deep: Int):
      inline def total: Int = ${ QuotedSum.identity('{ deep + own + value }) }
