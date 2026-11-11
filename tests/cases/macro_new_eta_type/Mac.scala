// Expected output captured with scalac 3.8.4.
import scala.quoted.*
class Box[A](val a: A)
class Carrier[F[_]]
type BoxLambda = [X] =>> Box[X]
type BoxAlias[X] = Box[X]
object Inspect:
  inline def result: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val direct = TypeRepr.of[Carrier[[X] =>> Box[X]]].typeArgs.head
    val alias = TypeRepr.of[Carrier[BoxLambda]].typeArgs.head
    val parameterized = TypeRepr.of[Carrier[BoxAlias]].typeArgs.head
    val constant = TypeRepr.of[Carrier[[X] =>> Box[Int]]].typeArgs.head
    Expr(New(Inferred(direct)).tpe.show + "\n" + New(Inferred(alias)).tpe.show + "\n" + New(Inferred(parameterized)).tpe.show + "\n" + New(Inferred(constant)).tpe.isInstanceOf[TypeLambda].toString)
