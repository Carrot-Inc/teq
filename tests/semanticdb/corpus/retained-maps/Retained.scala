import scala.annotation.StaticAnnotation
class Ann extends StaticAnnotation
object Retained {
  type Phantom[A] = Int
  def annotated[A](a: A): A @Ann = a
  def phantom[A](a: A): Phantom[A] = 1
  val annotatedInt = annotated[Int](1)
  val phantomString = phantom[String]("s")
}
