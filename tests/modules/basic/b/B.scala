package mb

import ma.*

class Sub extends Base(3):
  override def show: String = "sub " + n
object Main:
  def main(args: Array[String]): Unit =
    println(Rect(2.0).copy(h = 3.0))
    println(s"${Util.twice(top(1))}$topVal")
    println(new Sub().show)
