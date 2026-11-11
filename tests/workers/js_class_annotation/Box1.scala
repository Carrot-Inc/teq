package boxes

import scala.scalajs.js

class Box1(val n: Int) extends js.Object:
  def twice: Int = n * 2

object Use1:
  def make: Box1 = new Box1(1)
