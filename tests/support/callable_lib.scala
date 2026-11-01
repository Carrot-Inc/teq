// Library classes with a val named like a JavaScript function's own `name` and `length` (a val teq reads from TASTy
// through a getter), which a program's function-typed subclass inherits or overrides (tests/classpath/js/jar_val_callable.scala).
package callable

class Base:
  val name: String = "base"
  def show: String = "show:" + name

class BaseL:
  val length: Int = 1
