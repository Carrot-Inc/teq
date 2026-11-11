package boxes

import scala.scalajs.js

class Box0(val n: Int) extends js.Object:
  def twice: Int = n * 2

object Use0:
  def make: Box0 = new Box0(0)
