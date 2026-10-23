import scala.annotation.StaticAnnotation
class Ann extends StaticAnnotation
class C { type T }
class Inv[A]
object Ranges {
  def map[A, B](a: A)(f: A => B): B = f(a)
  val escaped = map(new C)(x => (??? : Inv[x.T @Ann]))
}
