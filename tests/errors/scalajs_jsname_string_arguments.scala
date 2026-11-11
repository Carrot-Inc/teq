// teq: tests/interop/scalajs-stub
// The String arguments of @JSName on nested native classes of a static owner: checkJSNativeLoadSpecOf's js.Symbol
// restriction reads `annot.arguments.head.tpe.derivesFrom(defn.StringClass)` (PrepJSInterop.scala:632), which a val
// of type String meets with no constant, and checkJSNameArgument's literal check reads `argumentConstantString(0)`,
// whose argument a cast is stripped from (`stripCast`, Annotations.scala:48). scalac 3.8.4 --js gives the one error
// below. An expectation's L:C is where scalac's caret begins (the start of dotty's span), 1-based.
// expect: 22:11: error: A String argument to JSName must be a literal string
// expect: 1 error found
// absent: Implementation restriction: @JSName with a js.Symbol is not supported on nested native classes and objects
// absent: 26:11: error: A String argument to JSName must be a literal string
import scala.scalajs.js
import scala.scalajs.js.annotation._

object Names {
  val s: String = "s"
}

@js.native
@JSGlobal
object Outer extends js.Object {
  @js.native
  @JSName(Names.s)
  class Inner extends js.Object

  @js.native
  @JSName("x".asInstanceOf[String])
  class Cast extends js.Object
}
