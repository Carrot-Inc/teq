// `derives` on a type constructor gives the instance for the constructor, as under scalac.
import scala.deriving.Mirror
trait Functor[F[_]]:
  def name: String
object Functor:
  inline def derived[F[_]](using m: Mirror.Of[F[Any]]): Functor[F] = new Functor[F]:
    def name = compiletime.constValue[m.MirroredLabel]
case class Box[A](a: A) derives Functor
@main def Main(): Unit =
  println(summon[Functor[Box]].name)
  println(summon[Mirror.Of[Box[Int]]].fromProduct(Tuple1(5)))
