package eia

object Lib:
  def twice(x: Int): Int = x * 2
  case class Luck(n: Int)
  given lucky: Luck = Luck(7)

trait Base:
  export eia.Lib.{*, given}
