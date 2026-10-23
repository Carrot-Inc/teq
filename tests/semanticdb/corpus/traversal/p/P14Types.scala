package p14
import scala.reflect.Selectable.reflectiveSelectable
object Types:
  type A = Int
  type F[X] = List[X]
  type L = [X] =>> Map[X, X]
  type B <: AnyRef
  opaque type O = Int
  def r: { def a: Int; val b: String } = ???
  def r2(x: { type T; def m: T }): x.T = x.m
  val e: List[?] = Nil
  def poly[T <: AnyVal, U[_]](t: T, u: U[T]): U[T] = u
  def local: Any =
    trait Q { type M }
    val q: Q { type M = Int } = ???
    def lf: { def z: Int } = ???
    q
  val inferred = new { def w = 1 }
