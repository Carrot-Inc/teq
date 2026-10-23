package nestedann
import scala.annotation.StaticAnnotation
class Box[A]
class Outer:
  class Ann extends StaticAnnotation
  def annotated: Box[Int @Ann] = ???
object Lib:
  val outer = new Outer
  def annotated: Box[Int @outer.Ann] = ???
