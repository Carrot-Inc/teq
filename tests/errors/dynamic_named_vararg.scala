import scala.language.dynamics
class Named extends Dynamic:
  def applyDynamicNamed(name: String)(args: (String, Any)*): String = name
  def applyDynamic(name: String)(args: Any*): String = name
def uses(d: Named, xs: Seq[Int]): Unit =
  println(d.call(a = 1, xs*))
  println(d.call(xs*))
  println(d.call(a = 1, b = 2))
// A spliced argument beside a named one in a call that is rewritten (`Dynamic.typedDynamicApply`); a splice alone
// and named arguments alone are accepted.
// expect: 6:11: error: applyDynamicNamed does not support passing a vararg parameter
// expect: 1 error found
