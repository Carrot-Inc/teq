package gb

import ga.*

case class Point(x: Int, y: Int)
given Show[Point] with
  def show(p: Point): String = "point " + p.x + "," + p.y

object Use:
  def main(args: Array[String]): Unit =
    println(summon[Show[Int]].show(1))
    println(describe(List(1, 2)))
    println(describe("s"))
    println(describe(Point(1, 2)))
