import scala.annotation.StaticAnnotation
class Ann extends StaticAnnotation
class Other extends StaticAnnotation
class C { type T }
class Inv[A]
object Nested {
  def map[A, B](a: A)(f: A => B): B = f(a)
  val one = map(new C)(x => (??? : Inv[x.T @Ann]))
  val two = map(new C)(x => (??? : Inv[x.T @Ann @Other]))
  val swapped = map(new C)(x => (??? : Inv[x.T @Other @Ann]))
}
