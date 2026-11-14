import scala.quoted.*
object Macros:
  // The copies of these blocks stand where the macros are called.
  inline def method(inline n: Int): Int = ${ methodImpl('n) }
  def methodImpl(n: Expr[Int])(using Quotes): Expr[Int] =
    '{
      def first: Int = second
      val held: Int = first
      def second: Int = $n
      held
    }
  inline def module(inline n: Int): Int = ${ moduleImpl('n) }
  def moduleImpl(n: Expr[Int])(using Quotes): Expr[Int] =
    '{
      def first: Int = Later.v
      val held: Int = first + $n
      object Later { val v: Int = 1 }
      held
    }
  inline def created(inline n: Int): Int = ${ createdImpl('n) }
  def createdImpl(n: Expr[Int])(using Quotes): Expr[Int] =
    '{
      def first: Int = new Later($n).v
      val held: Int = first
      class Later(val v: Int)
      held
    }
