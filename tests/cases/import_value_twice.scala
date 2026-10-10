// A value imported twice brings its conversion once: one method through one value is one candidate
// (scalac: "s1").
import scala.language.implicitConversions
class C { implicit def c(x: Int): String = "s" + x }
def f: String = {
  val v = new C
  import v.*
  import v.*
  1
}
@main def run(): Unit = println(f)
