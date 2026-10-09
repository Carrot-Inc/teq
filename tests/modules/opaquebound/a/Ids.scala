package oba

object Ids:
  opaque type Pos <: Int = Int
  object Pos:
    def apply(i: Int): Pos = i
    extension (p: Pos) def twice: Pos = p * 2
  opaque type Name <: String = String
  object Name:
    def apply(s: String): Name = s
  opaque type Money <: Long = Long
  object Money:
    def apply(l: Long): Money = l
    extension (m: Money) def plus(other: Money): Money = m + other + 1L
