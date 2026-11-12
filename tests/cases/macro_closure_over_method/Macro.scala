// `Closure(meth, None)` over a reference to a method is the method's function value, typed with the method's
// function type (`TypeAssigner.assignType` of a `Closure`); a member of a generic class as its receiver has it.
import scala.quoted.*

class Fns:
  def twice(n: Int): Int = n * 2

class Box[A](val a: A):
  def put(x: A): List[A] = List(a, x)

inline def twiceOf(inline f: Fns): Int => Int = ${ twiceOfImpl('f) }

def twiceOfImpl(f: Expr[Fns])(using q: Quotes): Expr[Int => Int] =
  import q.reflect.*
  Closure(Select.unique(f.asTerm, "twice"), None).asExprOf[Int => Int]

inline def putOf[A](inline b: Box[A]): A => List[A] = ${ putOfImpl[A]('b) }

def putOfImpl[A: Type](b: Expr[Box[A]])(using q: Quotes): Expr[A => List[A]] =
  import q.reflect.*
  Closure(Select.unique(b.asTerm, "put"), None).asExprOf[A => List[A]]
