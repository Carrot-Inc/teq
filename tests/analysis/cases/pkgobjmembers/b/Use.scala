package pomb

import poma.inner.{*, given}

// The package object's class and case class constructed, its val read and its given summoned, from another module.
object Use:
  def box: Int = new Box(3).twice
  def point: Point = Point(1, 2)
  def start: Int = origin.x
  def summoned: Int = summon[Box].n
