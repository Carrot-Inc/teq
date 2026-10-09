package cyc

object A:
  export B.*
  def a: Int = 1

object B:
  export A.*
  def b: Int = 2
