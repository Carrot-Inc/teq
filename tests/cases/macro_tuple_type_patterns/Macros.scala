package tlib
import scala.deriving.Mirror
import scala.quoted.*

object Fields:
  inline def describe[A](using m: Mirror.Of[A]): String = ${ describeImpl[A]('m) }

  def describeImpl[A: Type](m: Expr[Mirror.Of[A]])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def go[Ls: Type, Ts: Type](idx: Int): List[String] =
      (Type.of[Ls], Type.of[Ts]) match
        case ('[l *: ll], '[t *: tt]) =>
          val name = TypeRepr.of[l] match
            case ConstantType(StringConstant(n)) => n
            case _ => "?"
          s"$idx:$name:${Type.show[t]}" :: go[ll, tt](idx + 1)
        case ('[EmptyTuple], _) => Nil
    val fields = m match
      case '{ $p: Mirror.ProductOf[A] { type MirroredElemLabels = ls; type MirroredElemTypes = ts } } => go[ls, ts](0)
      case '{ $s: Mirror.SumOf[A] { type MirroredElemLabels = ls; type MirroredElemTypes = ts } } => "sum" :: go[ls, ts](0)
    Expr(fields.mkString(", "))
