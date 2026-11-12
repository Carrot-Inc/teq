// A closure over a method takes its receiver once, where the closure is made (the qualifier of the method that
// dotty's `Closure` node holds), not at each call of the function.
import scala.quoted.*

class Fns(val k: Int):
  def times(n: Int): Int = n * k

inline def timesOf(inline f: Fns): Int => Int = ${ timesOfImpl('f) }

def timesOfImpl(f: Expr[Fns])(using q: Quotes): Expr[Int => Int] =
  import q.reflect.*
  Closure(Select.unique(f.asTerm, "times"), None).asExprOf[Int => Int]

object Lib:
  println("Lib made")
  def twice(n: Int): Int = n * 2

inline def twiceOfLib: Int => Int = ${ twiceOfLibImpl }

def twiceOfLibImpl(using q: Quotes): Expr[Int => Int] =
  import q.reflect.*
  Closure(Select.unique('{ Lib }.asTerm, "twice"), None).asExprOf[Int => Int]
