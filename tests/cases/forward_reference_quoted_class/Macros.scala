import scala.quoted.*
// A creation of a class the quote's block declares later, over a value of the block: scalac's forward-reference
// check does not see the creation in the quote's copy, and the program runs.
object Macros:
  inline def created(inline n: Int): Int = ${ createdImpl('n) }
  def createdImpl(n: Expr[Int])(using Quotes): Expr[Int] =
    '{
      def first: Int = new Later($n).v
      val held: Int = first
      class Later(val v: Int)
      held
    }
  inline def direct(inline n: Int): Int = ${ directImpl('n) }
  def directImpl(n: Expr[Int])(using Quotes): Expr[Int] =
    '{
      val held: Int = new Later($n).v + 1
      class Later(val v: Int)
      held
    }
