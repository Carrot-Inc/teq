// An opaque type with an upper bound has the members, operators and supertypes of the bound
// outside its scope; an extension on the opaque type applies where the bound has no such member.
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

object Use:
  import Ids.*
  val q: Int = Pos(3) + 1
  val t: Pos = Pos(4).twice
  val n = Name("scala")
  val len: Int = n.length
  val up: String = n.toUpperCase
  val s: String = n
  val m: Long = Money(2L) + Money(3L)
  val mp: Money = Money(2L).plus(Money(3L))
  val cmp: Boolean = Pos(1) < Pos(2)

@main def run(): Unit =
  println(Use.q)
  println(Use.t)
  println(Use.len)
  println(Use.up)
  println(Use.s + "!")
  println(Use.m)
  println(Use.mp)
  println(Use.cmp)
