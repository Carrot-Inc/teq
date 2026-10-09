package pl

class PlBox(val v: Int):
  def removed: Int = v
  def plus(i: Int): Int = v + i
  def ++(i: Int): Int = v + i

object PlBox:
  def of(v: Int): PlBox = new PlBox(v)
