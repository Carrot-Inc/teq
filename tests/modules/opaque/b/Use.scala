package ob

import oa.Units.*

object Use:
  def total(ms: List[Meters]): Meters = ms.foldLeft(Meters(0.0))(_ + _)
  def main(args: Array[String]): Unit =
    println(total(List(Meters(1.0), Meters(2.5))).value)
