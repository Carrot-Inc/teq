// Under 3.5 two givens of one extension method, the derived owner's of the more general type: the intermediate
// scheme ties them, and the method read on the more specific given wins (dotty's `compareWithTypes`'
// `comparePrefixes`). scalac 3.8.4 prints `specific`.
import scala.language.`3.5`
class Ext(val tag: String):
  extension (s: String) def show: String = tag
class SubExt(tag: String) extends Ext(tag)
trait Low:
  given specific: SubExt = SubExt("specific")
object O extends Low:
  given general: Ext = Ext("general")
@main def run(): Unit =
  import O.given
  println("x".show)
