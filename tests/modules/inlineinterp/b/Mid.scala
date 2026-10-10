package iib

import iia.Lib

object Mid:
  inline def plain: String = Lib.text(2)
  def first: String = plain
  def second: String = Lib.text(3)
  def third: String = Lib.both(4, "four")
  def nums: List[Int] = Lib.list(4)
  def selected: Int = Lib.choice(true)
