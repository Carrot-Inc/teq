import scala.quoted.*
object Q:
  inline def twice[T: Type](e: Expr[T])(using Quotes): Expr[(T, T)] = '{ ($e, $e) }
  inline def typed[T: Type](using Quotes): String = Type.show[List[T]]
  inline def lifted(n: Int)(using Quotes): Expr[Int] = '{ ${ Expr(n) } * 10 }
  def pairImpl[T: Type](e: Expr[T])(using Quotes): Expr[String] =
    val p = twice(e)
    '{ $p.toString + " " + ${ Expr(typed[T]) } + " " + ${ lifted(4) } }
  inline def pair[T](inline x: T): String = ${ pairImpl('x) }
  def valueImpl(using Quotes): Expr[Int] = lifted(5)
  inline def value: Int = ${ valueImpl }
  inline def describe[T: Type](e: Expr[T])(using Quotes): String = e match
    case '{ ($a: Int) + 1 } => "plus one of " + a.show
    case '{ $x: T } => "a " + Type.show[T]
  def describeImpl[T: Type](e: Expr[T])(using Quotes): Expr[String] = Expr(describe(e))
  inline def d[T](inline x: T): String = ${ describeImpl('x) }
