package obb

import oba.Ids.*

object Use:
  val q: Int = Pos(3) + 1
  val t: Pos = Pos(4).twice
  val n = Name("scala")
  val len: Int = n.length
  val up: String = n.toUpperCase
  val s: String = n
  val m: Long = Money(2L) + Money(3L)
  val mp: Money = Money(2L).plus(Money(3L))
  val cmp: Boolean = Pos(1) < Pos(2)
