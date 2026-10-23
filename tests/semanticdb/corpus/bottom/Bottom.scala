import scala.annotation.StaticAnnotation
class Mark extends StaticAnnotation
class Context { type T }
class Box[A]
object Bottom {
  def map[A, B](a: A)(f: A => B): B = f(a)
  val result = map(new Context)(x => (??? : Box[(Nothing & String & x.T) @Mark]))
}
