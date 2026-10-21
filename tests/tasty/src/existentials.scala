// chimney-macro-commons' `Existentials` (2.2.0) in small, for the classpath case existential_bounded: a sealed
// constructor whose parameter's own parameter is bounded by its siblings (`F[_ >: L <: U]`), a case class extending it
// within its companion, aliases naming it through the trait's path, and a macro whose implementation builds and maps
// one at expansion, so that teq types these bodies from the pickles.
package fix.exist

import scala.quoted.*

trait ExbTypes:
  type Type[A]
  object Type:
    def apply[A](using t: Type[A]): Type[A] = t

trait ExbExistentials { this: ExbTypes =>
  sealed trait Existential[F[_]]:
    type Underlying
    val Underlying: Type[Underlying]
    val value: F[Underlying]
  object Existential:
    sealed trait Bounded[L <: U, U, F[_ >: L <: U]]:
      type Underlying >: L <: U
      val Underlying: Type[Underlying]
      val value: F[Underlying]
      def mapK[G[_ >: L <: U]](f: Type[Underlying] ?=> F[Underlying] => G[Underlying]): Bounded[L, U, G] =
        Bounded[L, U, G, Underlying](f(using Underlying)(value))(using Underlying)
    object Bounded:
      final case class Impl[L <: U, U, F[_ >: L <: U], A >: L <: U](Underlying: Type[A], value: F[A]) extends Bounded[L, U, F]:
        type Underlying = A
      def apply[L <: U, U, F[_ >: L <: U], A >: L <: U: Type](value: F[A]): Bounded[L, U, F] = Impl[L, U, F, A](Type[A], value)
    type UpperBounded[U, F[_ <: U]] = Bounded[Nothing, U, F]
  type ExistentialType = Existential.UpperBounded[Any, Type]
}

object ExbImpl extends ExbTypes with ExbExistentials:
  type Type[A] = String
  given [A]: Type[A] = "t"
  def describe(n: Int): String =
    val b = Existential.Bounded[Nothing, Any, List, Int](List(n))
    val c = b.mapK[Option](l => l.headOption)
    s"bounded ${c.value} ${c.Underlying}"

object ExbMac:
  inline def bounded(inline n: Int): String = ${ boundedImpl('n) }
  def boundedImpl(n: Expr[Int])(using Quotes): Expr[String] = Expr(ExbImpl.describe(n.valueOrAbort))
