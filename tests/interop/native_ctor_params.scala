// A plain constructor parameter of a native class is no field, so a member may share its name,
// as Scala.js facades write it.
import scala.scalajs.js
import scala.scalajs.js.annotation.*

@js.native @JSGlobal("Int8Array")
class Int8Array(length: Int) extends js.Object:
  def length: Int = js.native
  @JSBracketAccess def apply(index: Int): Int = js.native
  @JSBracketAccess def update(index: Int, value: Int): Unit = js.native

@js.native @JSGlobal("Error")
class JsError(message: String) extends js.Object:
  val message: String = js.native
  val name: String = js.native

@js.native @JSGlobal("Map")
class JsMap(entries: js.Array[js.Array[String]] = js.native) extends js.Object:
  def size: Int = js.native
  def entries(): js.Object = js.native
  def get(key: String): js.UndefOr[String] = js.native

@main def main(): Unit =
  val bytes = new Int8Array(4)
  bytes(1) = 7
  println(bytes.length)
  println(bytes(1))
  val error = new JsError("boom")
  println(error.message + " " + error.name)
  val entries = js.Array(js.Array("a", "1"), js.Array("b", "2"))
  val map = new JsMap(entries)
  println(map.size)
  println(map.get("b"))
  println(new JsMap().size)
