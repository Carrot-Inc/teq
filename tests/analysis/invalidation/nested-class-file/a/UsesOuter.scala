package nca

object UsesOuter:
  def v: Int = new Outer().make.v
