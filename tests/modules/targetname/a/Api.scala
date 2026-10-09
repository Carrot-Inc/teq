package tna

import scala.annotation.targetName

case class Vec(x: Int, y: Int):
  @targetName("plus") def +(o: Vec): Vec = Vec(x + o.x, y + o.y)
  @targetName("scaled") def *(k: Int): Vec = Vec(x * k, y * k)
