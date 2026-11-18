// Where a class has its own `==` beside `Any`'s, several arguments are tupled for `Any`'s only where every alternative
// is unary (dotty's `isUnary`, `needsTupledDual`): a class's own `==` of two parameters leaves three arguments to a
// conversion, a nullary one leaves two to an extension, and one taking a tuple takes them on a type parameter's
// receiver. Without auto-tupling (`noAutoTupling`) nothing takes two arguments but a conversion's class.
import scala.language.implicitConversions
class P { def ==(x: String, y: String): String = "two" }
class Three { def ==(a: Int, b: Int, c: Int): String = "three" }
object Dsl { implicit def conv(p: P): Three = new Three }
class Q { def ==(): String = "nullary" }
extension (q: Q) def ==(a: Int, b: Int): String = "ext2"
class T2 { def ==(x: (String, String)): String = "own-tuple" }
def tupled[T <: T2](t: T): String = t.==("a", "b")
class Payload { def ==(x: String, y: String): String = "equal" }
object Wrap { implicit def conv(b: Boolean): Payload = new Payload }
def three(): String = { import Dsl.*; new P().==(1, 2, 3) }
def untupled(): String = { import scala.language.noAutoTupling; import Wrap.*; true.==("a", "b") }
@main def run(): Unit =
  println(three())
  println(new Q().==(1, 2))
  println(tupled(new T2))
  println(untupled())
