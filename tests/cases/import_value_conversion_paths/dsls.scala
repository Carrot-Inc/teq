package dsls
import scala.language.implicitConversions

class Dsl(tag: String):
  implicit def asString(i: Int): String = s"$tag:$i"
  def name: String = tag

object Config:
  val dsl = new Dsl("object")

val pkgDsl = new Dsl("package")
