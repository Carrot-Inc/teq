//> using platform js
import scala.scalajs.js
import scala.language.dynamics
class Hook extends js.Object with scala.Dynamic:
  def selectDynamic(s: String): String = "hook:" + s
  def applyDynamic(s: String)(n: Int): String = "call:" + s + ":" + n
@main def run(): Unit =
  val d = new Hook
  println(d.x)
  println(d.f(3))
  println(d.selectDynamic("written"))
