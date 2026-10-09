// Item 5 of the JVM ABI alignment: a method's `@targetName` is
// its name in the class files, overloaded or not, its bridges', its static and its trait forwarders' too, and its
// default getters keep the source name (`$plus$default$1`); a right-associative extension takes the method's first
// explicit clause before the receiver's (`$plus$colon(String, int)`).
// A right-associative extension's defaults are numbered and take the earlier clauses as its declaration orders them.
// abi: renamed renamed$ $plus $plus$default$1 $plus$default$1$ renamed$default$1 str single g $plus$colon $plus$plus$colon $colon$colon$colon $plus$colon$default$1 $plus$plus$colon$default$3
package target_names
import scala.annotation.targetName
trait P:
  @targetName("renamed") def f(x: String): String
trait T extends P:
  @targetName("renamed") def f(x: String): String = x
class C extends T:
  @targetName("renamed") override def f(x: String): String = x * 2
object TN:
  @targetName("renamed") def +(x: Int = 1): Int = x
  @targetName("str") def +(x: String): String = x
  @targetName("single") def g(x: Int): Int = x
trait TT:
  @targetName("renamed") def +(x: Int = 1): Int = x
object Syntax:
  extension (n: Int) def +:(s: String): String = s + n
  extension (n: Int) def ++:(s: String)(using sep: Char): String = s + sep + n
  extension [A](x: A) def :::(xs: List[A]): List[A] = x :: xs
object RightDefaults:
  extension (n: Int) def +:(s: String = "x"): String = s * n
  extension (n: Int) def ++:(s: String)(k: Int = 2): String = (s * n) * k
