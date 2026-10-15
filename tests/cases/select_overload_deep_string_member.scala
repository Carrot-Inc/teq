// The methods of `java.lang.String` std/javalib/string.scala gives the builtin `String` are its own members in a
// selection, as dotty's `member` finds them on the class, `@javaDefined` or not: `charAt(1)` takes the string's,
// `charAt("x")` `C`'s.
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def charAt(s: String): Int = 9 }
object S:
  def g(using T): C = new C
  def g(using U): String = "abc"
@main def run(): Unit =
  println(S.g.charAt("x"))
  println(S.g.charAt(1))
