//> using platform js
// `js.Dictionary[A]` is a `js.Any` and no `js.Object`, as Scala.js declares it: a given for
// `js.Object` is no candidate for a dictionary (scalac --js: "dict", "keys a,b").
import scala.scalajs.js
final case class ValueType[-A, +U](name: String)
object VT:
  type Simple[A] = ValueType[A, A]
object Implicits:
  import VT.Simple
  implicit val vtObj: Simple[js.Object] = ValueType("obj")
  implicit def vtDict[A]: ValueType[js.Dictionary[A], js.Object] = ValueType("dict")
import Implicits.*
@main def main(): Unit =
  println(summon[ValueType[js.Dictionary[String], js.Object]].name)
  val d = js.Dictionary("a" -> 1, "b" -> 2)
  println(s"keys ${d.keys.mkString(",")}")
