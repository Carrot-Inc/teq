// teq: tests/interop/scalajs-stub
// PrepJSInterop's and PrepJSExports' checks: scalac 3.8.4 --js types the program, so the phase runs (Phase.isRunnable).
// An expectation's L:C is where scalac's caret begins (the start of dotty's span), 1-based.
// expect: 25:18: error: Concrete members of JS native types may only call js.native.
// absent: 26:18: error: a default argument of a native JS method has to be js.native or js.undefined
// expect: 28:1: error: @JSImport can only be used on native JS definitions (with @js.native).
// expect: 31:12: error: @JSGlobalScope can only be used on native JS objects (with @js.native).
// expect: 34:1: error: Native JS classes, vals and defs must have exactly one annotation among @JSGlobal and @JSImport.
// expect: 37:32: error: Native JS objects must have exactly one annotation among @JSGlobal, @JSImport and @JSGlobalScope.
// expect: 42:3: error: Native JS objects cannot contain inner non-native JS classes or objects
// expect: 46:3: error: Members of non-native JS traits must either be abstract, or their right-hand-side must be `js.undefined`.
// expect: 47:3: error: Members of non-native JS traits must either be abstract, or their right-hand-side must be `js.undefined`.
// expect: 54:3: error: @JSImport can only be used on native JS definitions (with @js.native).
// expect: 57:3: error: The top-level export name must be a valid JavaScript identifier name
// expect: 60:3: error: @JSName can only be used on members of JS types.
// expect: 63:3: error: @JSGlobalScope can only be used on native JS objects (with @js.native).
// expect: 12 errors found
package app

import scala.scalajs.js
import scala.scalajs.js.annotation.*

@js.native
trait NativeTrait extends js.Object:
  def f(): Int = 1
  def g(x: Int = 5): Int = js.native

@JSImport("m", "x")
object NoNative extends js.Object

@js.native @JSGlobalScope
class ScopeClass extends js.Object

@js.native
class Unbound extends js.Object

@js.native @JSImport("m", "n") @JSGlobal("g")
object TwoBindings extends js.Object

@js.native @JSGlobal("Outer")
object Outer extends js.Object:
  class Nested extends js.Object

trait JsTrait extends js.Object:
  val name: String
  def g: Int = 1
  val v: Int = 3

class Plain(val a: Int) extends js.Object

class Sub extends Plain(1)

object Facade:
  @JSImport("m", "n")
  def h(x: Int): Int = x + 1

  @JSExportTopLevel("bad name")
  val z: Int = 1

  @JSName("y")
  val y: Int = 1

  @JSGlobalScope
  def scoped: Int = js.native
