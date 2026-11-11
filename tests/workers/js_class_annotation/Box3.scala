package boxes

import scala.scalajs.js

class Box3(val n: Int) extends js.Object:
  def twice: Int = n * 2

object Use3:
  def make: Box3 = new Box3(3)
