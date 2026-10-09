package ita

class Box:
  type T
  val v: Int = 1

object A:
  type T = Int
  def m: Int = 1

object B:
  export A.{T => U, m => n}
