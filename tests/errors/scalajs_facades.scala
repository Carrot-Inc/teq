// teq: tests/interop/scalajs-stub
// expect: 35:18: error: f is a member of a native JS type: it has to be abstract or have the body js.native
// expect: 36:18: error: a default argument of a native JS method has to be js.native or js.undefined
// expect: 38:7: error: @JSBracketAccess goes on apply (a read) or update (a write)
// expect: 41:8: error: @JSImport, @JSGlobal and @JSGlobalScope need @js.native on the class or object
// expect: 44:7: error: @JSGlobalScope is only allowed on an object
// expect: 47:7: error: the native JS class Unbound needs @JSImport, @JSGlobal or @JSGlobalScope to say where JavaScript defines it
// expect: 50:8: error: a class or object can only have one of @JSImport, @JSGlobal and @JSGlobalScope
// expect: 54:9: error: Nested is nested in a native JS type, which cannot define classes or objects
// expect: 58:7: error: g is a member of a JS trait, which can only hold abstract members and vals set to js.undefined
// expect: 59:7: error: v is a member of a JS trait, which can only hold abstract members and vals set to js.undefined
// expect: 67:7: error: the body of a @JSImport or @JSGlobal definition has to be js.native
// expect: 70:7: error: @JSImport needs a module and a name: @JSImport("module", "name"), JSImport.Default or JSImport.Namespace
// expect: 73:7: error: @JSExportTopLevel needs a valid JS identifier: @JSExportTopLevel("name")
// expect: 76:7: error: @JSName is only allowed on a member of a JS type
// expect: 79:7: error: @JSGlobalScope is only allowed on an object
// expect: 82:5: error: selectDynamic takes the property name as its one argument
// expect: 83:5: error: js.Dynamic.literal takes either named arguments or pairs, not both
// expect: 86:5: error: the anonymous JS object does not implement abstract member(s): name
// expect: 87:19: error: reassignment to val name
// expect: 88:5: error: the anonymous JS object does not implement abstract member(s): name
// expect: 88:23: error: value other is not a member of JsTrait
// expect: 89:5: error: the anonymous JS object does not implement abstract member(s): name
// expect: 89:23: error: m cannot be defined in an anonymous JS object, which holds vals and assignments to the vars of its trait; write a class for methods
// expect: 92:5: error: NativeTrait is a JS trait, which cannot be tested at runtime; test a native class or a property instead
// expect: 92:36: error: JsTrait is a JS trait, which cannot be tested at runtime; test a native class or a property instead
// expect: 25 errors found
package app

import scala.scalajs.js
import scala.scalajs.js.annotation.*

@js.native
trait NativeTrait extends js.Object:
  def f(): Int = 1
  def g(x: Int = 5): Int = js.native
  @JSBracketAccess
  def get(i: Int): Int = js.native

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

  @JSImport("m", Facade.h)
  def k: Int = js.native

  @JSExportTopLevel("bad name")
  val z: Int = 1

  @JSName("y")
  val y: Int = 1

  @JSGlobalScope
  def scoped: Int = js.native

  def literalForms(d: js.Dynamic): Unit =
    d.selectDynamic("a", "b")
    js.Dynamic.literal(a = 1, "b" -> 2)

  def anonymous(): Unit =
    new JsTrait {}
    new JsTrait { name = "x" }
    new JsTrait { val other = 1 }
    new JsTrait { def m = 1 }

  def tests(x: Any): Boolean =
    x.isInstanceOf[NativeTrait] || x.isInstanceOf[JsTrait]
