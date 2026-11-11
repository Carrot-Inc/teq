package boxes

import scala.scalajs.js

class Box7(val n: Int) extends js.Object:
  def twice: Int = n * 2

object Use7:
  def make: Box7 = new Box7(7)
