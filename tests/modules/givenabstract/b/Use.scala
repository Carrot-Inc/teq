package gab

import gaa.Decls

class ByGiven extends Decls:
  given x: Int = 5
  given label(using n: Int): String = "g" + n
class ByDef extends Decls:
  def x: Int = 6
  given label(using n: Int): String = "d" + n

object Main:
  def main(args: Array[String]): Unit =
    for d <- List(ByGiven(), ByDef()) do
      given Int = d.x
      println(s"${d.x} ${d.twice} ${d.label}")
