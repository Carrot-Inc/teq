package pl

class PlBox(val v: Int)

object PlBox:
  def of(v: Int): PlBox = new PlBox(v)
