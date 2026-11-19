// A class that writes no annotation reads `js.Tuple2`, whose std members carry annotations: completing them
// on demand reads their annotations' classes, which records no used name for this class.
import scala.scalajs.js

object Main:
  def first(c: js.Tuple2[Int, Int]): Int = c._1
