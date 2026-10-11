package cpa

object Scope:
  case class C(n: Int)
  case class Pair[A](a: A, n: Int)

case class Top(s: String)
