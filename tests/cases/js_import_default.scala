//> using platform js
//> using jsModuleKind es
//> using jsVersion 1.21.0
// JSImport.Default reads `default` off the module's namespace, so a module without a default
// export loads and the facade reads undefined; the default function is called without a receiver.
import scala.scalajs.js
import scala.scalajs.js.annotation.JSImport

@js.native
@JSImport("data:text/javascript,export default class Counter { constructor(n) { this.n = n } next() { return ++this.n } }", JSImport.Default)
class Counter(start: Int) extends js.Object:
  def next(): Int = js.native

@js.native
@JSImport("data:text/javascript,export default function () { return this === undefined }", JSImport.Default)
def unbound(): Boolean = js.native

@js.native
@JSImport("data:text/javascript,export default { greeting: 'hi' }", JSImport.Default)
val settings: js.Dynamic = js.native

@js.native
@JSImport("data:text/javascript,export const css = 'a {}'", JSImport.Default)
val stylesheet: js.Any = js.native

@js.native
@JSImport("data:text/javascript,export default { twice: x => 2 * x }", JSImport.Default)
object Maths extends js.Object:
  def twice(x: Int): Int = js.native

@main def run(): Unit =
  val c = new Counter(4)
  c.next()
  println(c.next())
  println(unbound())
  println(settings.greeting)
  println(js.isUndefined(stylesheet))
  println(Maths.twice(21))
