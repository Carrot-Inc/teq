// An export of a class above implements an abstract member only where its result conforms: scalac
// refuses `Child` ("has incompatible type"), and so does teq, while `Fine`'s export, of a conforming
// result, implements `GAPI.g` on the JVM, whose class files carry the forwarder `Fine` inherits
// (JavaScript refuses both).
// expect: 17:7: error: class Child needs to be abstract, since def f(x: Int): Int in trait API is not defined
// expect: 1 error found
// teq: --target jvm
object Impl:
  def f(x: Int): String = x.toString
  def g(x: Int): Int = x
trait Parent:
  export Impl.{f, g}
trait API:
  def f(x: Int): Int
trait GAPI:
  def g(x: Int): Int
class Child extends Parent with API
class Fine extends Parent with GAPI
