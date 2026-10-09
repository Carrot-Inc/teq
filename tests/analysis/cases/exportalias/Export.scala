package exal

object A:
  type T = Int
  type V = String

object B:
  export A.{T => U}

// A selection through an export's alias: the forwarder of B and the original of A.
class Use:
  def f(x: B.U): B.U = x
