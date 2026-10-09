package capture.js

import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobal

@js.native
@JSGlobal("Math")
object JsMath extends js.Object:
  def max(xs: Double*): Double = js.native

trait Named extends js.Object:
  val name: String

object Uses:
  def dynamic(d: js.Dynamic): js.Dynamic = d.inner.call(1)
  def update(d: js.Dynamic): Unit = d.name = "x"
  def literal: js.Dynamic = js.Dynamic.literal(a = 1, b = "two")
  def anonymous: Named = new Named { val name = "n" }
  def facade: Double = JsMath.max(1, 2, 3)
