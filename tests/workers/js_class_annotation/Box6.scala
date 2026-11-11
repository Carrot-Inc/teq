package boxes

import scala.scalajs.js

class Box6(val n: Int) extends js.Object:
  def twice: Int = n * 2

object Use6:
  def make: Box6 = new Box6(6)
