package boxes

import scala.scalajs.js

class Box5(val n: Int) extends js.Object:
  def twice: Int = n * 2

object Use5:
  def make: Box5 = new Box5(5)
