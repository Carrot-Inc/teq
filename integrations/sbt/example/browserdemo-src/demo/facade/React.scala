package demo.facade

import scala.scalajs.js
import scala.scalajs.js.annotation.*

@js.native @JSImport("react", JSImport.Namespace)
object React extends js.Object:
  def createElement(tpe: js.Any, props: js.Any, children: js.Any*): js.Any = js.native
  def useState(initial: js.Any): js.Array[js.Any] = js.native

@js.native @JSImport("react-dom/client", JSImport.Namespace)
object ReactDOMClient extends js.Object:
  def createRoot(container: js.Any): Root = js.native

@js.native
trait Root extends js.Object:
  def render(element: js.Any): Unit = js.native
