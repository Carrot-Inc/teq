// At the default source two Scala 2 implicits of one extension method, the derived owner's of the more general
// type: compared by the intermediate scheme they tie, and the method read on the more specific one wins (dotty's
// `compareWithTypes`' `comparePrefixes`). scalac 3.8.4 prints `specific`.
class Ext(val tag: String):
  extension (s: String) def show: String = tag
class SubExt(tag: String) extends Ext(tag)
trait Low:
  implicit val specific: SubExt = SubExt("specific")
object O extends Low:
  implicit val general: Ext = Ext("general")
@main def run(): Unit =
  import O.*
  println("x".show)
