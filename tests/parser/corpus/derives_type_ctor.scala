// `derives TC` on a type constructor where `TC` takes one: the instance is `TC[C]`, from a
// mirror of the constructor whose element types are a lambda over its parameters, as scalac
// synthesizes it.
import scala.deriving.Mirror
import scala.compiletime.*
trait Arity[F[_]]:
  def arity: Int
  def label: String
object Arity:
  type Of[F[_]] = Mirror.Product { type MirroredType = F; type MirroredMonoType = F[Any]; type MirroredElemTypes[_] <: Tuple; type MirroredLabel <: String }
  inline def derived[F[_]](using m: Of[F]): Arity[F] =
    val n = constValue[Tuple.Size[m.MirroredElemTypes[Any]]]
    val l = constValue[m.MirroredLabel & String]
    new Arity[F]:
      def arity = n
      def label = l
case class Pair[A](first: A, second: A) derives Arity
case class Tree[A](value: A, children: List[Tree[A]]) derives Arity
case class Box[A](a: A) derives Arity
object Main:
  def show[F[_]](using a: Arity[F]): String = a.label + "/" + a.arity
  def main(args: Array[String]): Unit =
    println(show[Pair] + " " + show[Tree] + " " + show[Box])
