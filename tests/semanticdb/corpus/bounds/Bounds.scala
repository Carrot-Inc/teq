import scala.annotation.StaticAnnotation
class Ann extends StaticAnnotation
class Other extends StaticAnnotation
class Third extends StaticAnnotation
class C { type T }
class D { type T >: Int @Ann <: Int @Other }
class E { type T >: String @Ann <: Any @Other }
class Inv[A]
object Bounds {
  def map[A, B](a: A)(f: A => B): B = f(a)
  val eq = map(new D)(x => (??? : Inv[x.T]))
  val eqAnn = map(new D)(x => (??? : Inv[x.T @Third]))
  val lower = map(new E)(x => (??? : Inv[x.T @Third]))
  val triple = map(new C)(x => (??? : Inv[x.T @Ann @Other @Third]))
}
