//> using platform js
// `js.constructorOf[C]` is the JavaScript class value of a native class: here the global `Error`,
// which a function made a subclass of through `js.Object.create` extends.
import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobal

@js.native
@JSGlobal("Error")
class JsError(message: String) extends js.Object

@main def run(): Unit =
  val parent = js.constructorOf[JsError]
  println(parent == js.Dynamic.global.Error)
  var Ctor: js.ThisFunction1[js.Dynamic, String, js.Dynamic] = null
  Ctor = (self: js.Dynamic, name: String) => {
    self.name = name
    self
  }
  val proto = js.Object.create(parent.prototype.asInstanceOf[js.Object])
  Ctor.asInstanceOf[js.Dynamic].prototype = proto
  val o = js.Dynamic.newInstance(Ctor.asInstanceOf[js.Dynamic])("widget")
  println(o.name)
  println(js.special.instanceof(o, parent))
  println(js.Object.getPrototypeOf(js.Object.create(proto, js.Dynamic.literal())) == proto)
