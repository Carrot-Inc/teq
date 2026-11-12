// Pending on teq's lifting (queued; master the same): a local def reading `super`, called from an anonymous class.
// dotty's LambdaLift lifts the def into the declaring class, where the `invokespecial` stays; teq copies the def into
// the reading class, which cannot make the call (refused: "this super call", or a VerifyError for a trait's member).
// The expectation is scalac 3.8.4's.
import java.util.concurrent.Callable
class Parent { def value: Int = 21 }
trait T { def label(s: String): String = s"T $s" }
class Outer extends Parent {
  override def value: Int = 42
  def viaDef(): Int = {
    def x: Int = super.value
    new Callable[Int] { def call(): Int = x }.call()
  }
}
class Mixer extends T {
  override def label(s: String): String = s"Mixer $s"
  def viaDef(): String = {
    def x: String = super.label("def")
    new Callable[String] { def call(): String = x }.call()
  }
}
object Main:
  def main(args: Array[String]): Unit =
    println(new Outer().viaDef())
    println(new Mixer().viaDef())
