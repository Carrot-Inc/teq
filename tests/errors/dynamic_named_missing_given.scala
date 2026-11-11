// expect: 11:27: error:
// expect: 1 error found
// A named Dynamic call's missing given is reported at the end of its arguments: applyDynamicNamed's Apply is the
// envelope of its tuples, each the value's (Dynamic.scala:89-95, Positioned.scala:89-126).
import scala.language.dynamics
trait Evidence
object D extends Dynamic:
  def applyDynamicNamed(name: String)(xs: (String, Int)*)
      (using Evidence): Int = 0

val bad: Int = D.foo(x = 1)
