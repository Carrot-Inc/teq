// The standard interpolators called on the `StringContext` its companion's `apply` makes of
// literal parts are linted as their syntax is (`StringInterpolatorOpt`'s `StringContextApply`,
// `StringContextIntrinsic`, `transformF`), however the source names them (an alias, `apply`) and
// a constant member among the parts; a spread argument, a part that is no constant, an instance
// made by `new`, a context held in a value and another method are not.
import scala.{StringContext as SC}
class C { override def toString = "c" }
object Parts { final val empty = "" }
object Main {
  val parts = Seq("", "")
  def main(args: Array[String]): Unit = {
    println(StringContext("", "").s(new C))
    println(StringContext("a", "b").raw(new C))
    println(StringContext("", "%s").f(new C))
    println(SC("", "").s(new C))
    println(StringContext.apply("", "").raw(new C))
    println(StringContext(Parts.empty, "").s(new C))
    println(StringContext("", "").s(Seq(new C)*))
    println(StringContext(parts*).s(new C))
    println(new StringContext("", "").s(new C))
    val sc = StringContext("", "")
    println(sc.s(new C))
    println(StringContext("", "").s(1))
    println(s"${new C}")
  }
}
