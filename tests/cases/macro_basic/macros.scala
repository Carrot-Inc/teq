import scala.quoted.*

object Macros:
  inline def answer: Int = ${ answerImpl }
  def answerImpl(using Quotes): Expr[Int] = '{ 42 }

  inline def twice(inline x: Int): Int = ${ twiceImpl('x) }
  def twiceImpl(x: Expr[Int])(using Quotes): Expr[Int] = '{ $x + $x }

  inline def isConst(inline x: Int): String = ${ isConstImpl('x) }
  def isConstImpl(x: Expr[Int])(using Quotes): Expr[String] =
    x.value match
      case Some(v) => Expr("constant " + v)
      case None => Expr("not a constant: " + x.show)

  inline def showType[T]: String = ${ showTypeImpl[T] }
  def showTypeImpl[T: Type](using Quotes): Expr[String] = Expr(Type.show[T])

  inline def sum(inline xs: Int*): Int = ${ sumImpl('xs) }
  def sumImpl(xs: Expr[Seq[Int]])(using Quotes): Expr[Int] =
    xs match
      case Varargs(elems) =>
        elems.foldLeft('{ 0 })((acc, e) => '{ $acc + $e })
      case _ => '{ $xs.sum }

  inline def summonShow[T]: String = ${ summonShowImpl[T] }
  def summonShowImpl[T: Type](using Quotes): Expr[String] =
    Expr.summon[Show[T]] match
      case Some(s) => '{ $s.show }
      case None => Expr("no Show for " + Type.show[T])

trait Show[T]:
  def show: String
object Show:
  given Show[Int] with
    def show: String = "Show[Int]"
