import scala.quoted.*

object Macros:
  inline def describe(inline x: Any): String = ${ describeImpl('x) }
  def describeImpl(x: Expr[Any])(using Quotes): Expr[String] =
    x match
      case '{ ($a: Int) + ($b: Int) } => '{ "sum of " + ${ describeImpl(a) } + " and " + ${ describeImpl(b) } }
      case '{ $n: Int } => n.value match
        case Some(v) => Expr("int " + v)
        case None => Expr("some int")
      case '{ $s: String } => Expr("a string")
      case '{ ($xs: List[t]).head } => '{ "head of a list of " + ${ Expr(Type.show[t]) } }
      case '{ $xs: List[t] } => '{ "list of " + ${ Expr(Type.show[t]) } }
      case _ => Expr("something else: " + x.show)

  inline def typeName[T]: String = ${ typeNameImpl[T] }
  def typeNameImpl[T: Type](using Quotes): Expr[String] =
    Type.of[T] match
      case '[Int] => Expr("Int")
      case '[List[t]] => '{ "List of " + ${ typeNameImpl[t] } }
      case '[Option[t]] => '{ "Option of " + ${ typeNameImpl[t] } }
      case '[(a, b)] => '{ "pair of " + ${ typeNameImpl[a] } + " and " + ${ typeNameImpl[b] } }
      case _ => Expr("other: " + Type.show[T])

  inline def lift(inline xs: List[Int]): List[Int] = ${ liftImpl('xs) }
  def liftImpl(xs: Expr[List[Int]])(using Quotes): Expr[List[Int]] =
    xs.value match
      case Some(list) => Expr(list.map(_ * 10))
      case None => xs

  inline def pairOf(inline a: Int, inline b: String): (Int, String) = ${ pairImpl('a, 'b) }
  def pairImpl(a: Expr[Int], b: Expr[String])(using Quotes): Expr[(Int, String)] =
    (a.value, b.value) match
      case (Some(x), Some(y)) => Expr((x + 1, y + "!"))
      case _ => '{ ($a, $b) }

  inline def optionOf(inline x: Option[Int]): String = ${ optionImpl('x) }
  def optionImpl(x: Expr[Option[Int]])(using Quotes): Expr[String] =
    x.value match
      case Some(Some(v)) => Expr("some " + v)
      case Some(None) => Expr("none")
      case None => Expr("unknown")
