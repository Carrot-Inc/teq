// A type variable bounded only through an inner application's variable (the two unified) and
// used only contravariantly in the result is maximised to `Any`, as scalac interpolates it:
// zio's `a ++ ZLayer.scoped(z)` is a `ZLayer[Any, ..]`, not a `ZLayer[Nothing, ..]`.
trait Scope
final class Z[-R, +E, +A](val run: () => A)
final class L[-R, +E, +O](val build: () => List[Any]):
  def ++[E1 >: E, R2, O2](that: => L[R2, E1, O2]): L[R & R2, E1, O & O2] = L(() => build() ++ that.build())
object L:
  def succeed[O](o: O): L[Any, Nothing, O] = L(() => List(o))
  def scoped[R]: Scoped[R] = Scoped[R]()
  final class Scoped[R]:
    def apply[E, A](z: => Z[Scope & R, E, A]): L[R, E, A] = L(() => List(z.run()))
final case class A(v: Int)
final case class B(v: String)
final case class C(v: Long)
def mkB: Z[Scope, Throwable, B] = Z(() => B("b"))
def needs(l: L[Any, Throwable, A & B]): String = l.build().mkString(", ")

@main def run(): Unit =
  val a: L[Any, Nothing, A] = L.succeed(A(1))
  val env = a ++ L.scoped(mkB)
  val any: L[Any, Throwable, A & B] = env
  println(needs(env))
  val env3 = a ++ L.scoped(mkB) ++ L.succeed(C(3L))
  val any3: L[Any, Throwable, A & B & C] = env3
  println(any3.build().size)
