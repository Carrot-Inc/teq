package loaded
import scala.annotation.StaticAnnotation
class Ann extends StaticAnnotation
class Box[A]
object Lib {
  type A = Box[Int]
  def annotated: Box[Int @Ann] = ???
  def aliased: A = ???
}
