package boxes

import scala.scalajs.js

class Box2(val n: Int) extends js.Object:
  def twice: Int = n * 2

object Use2:
  def make: Box2 = new Box2(2)
