// Types in signatures: applied, bounds, lambdas, refinements, match types, dependent method
// types, unions and intersections, singleton and path types, function types, by-name,
// higher-kinded parameters, context bounds.
package probe.types

import scala.annotation.targetName

trait Box[F[_]]:
  def wrap[A](a: A): F[A]
object Types:
  def union(x: Int | String): Int & Any = 1
  def lambda[F[_]]: Box[[X] =>> Either[String, X]] = null
  def refined(x: AnyRef { def size: Int }): Int = 0
  def dep(b: Container)(x: b.Elem): b.Elem = x
  def single(x: Types.type): x.type = x
  def fn(f: (Int, String) => Boolean, g: Int ?=> String): Unit = ()
  def ctx[A: Ordering](a: A): A = a
  def hk[F[_] <: Iterable[?], A](fa: F[A]): Int = 0
  def lit: 42 = 42
  def tuple(t: (Int, String)): Int = t._1
  def arr(a: Array[Int]): Array[String] = null
  @targetName("plusInt") def +(x: Int): Int = x
  type Elem[X] = X match
    case String => Char
    case List[t] => t
  def matchT[X](x: X): Elem[X] = ???
trait Container:
  type Elem
