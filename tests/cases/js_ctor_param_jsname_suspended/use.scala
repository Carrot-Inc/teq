//> using platform js
import scala.scalajs.js
import scala.scalajs.js.annotation.JSName

class C(@JSName("apply") val apply: Int) extends js.Object

object Use:
  def n: Int = M.one

@main def run(): Unit = println(Use.n + new C(2).apply)
