package boxes

import scala.scalajs.js

class Box4(val n: Int) extends js.Object:
  def twice: Int = n * 2

object Use4:
  def make: Box4 = new Box4(4)
