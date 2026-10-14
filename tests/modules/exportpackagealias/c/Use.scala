package app

import pre.Prelude.*

// Context bounds through the prelude's type forwarders of renaming aliases (`type Eq[A] =
// lib.kernel.Eq[A]`) and of aliases that are no renaming, a package object's and an object's.
object Use:
  given Eq[Int] = lib.kernel.Eq.instance(_ == _)
  given EqPair[Int] = lib.kernel.Eq.instance((x, y) => x == y)
  def same[T: Eq](a: T, b: T): Boolean = summon[Eq[T]].eqv(a, b)
  def pairs[T: EqPair](a: (T, T)): Boolean = summon[EqPair[T]].eqv(a, a)
  def alike[T: Same](a: T, b: T): Boolean = summon[Same[T]].eqv(a, b)
  def pairsAlike[T: SamePair](a: (T, T)): Boolean = summon[SamePair[T]].eqv(a, a)
  val shown: display = "block"
  val wide: columns = 3
  def main(args: Array[String]): Unit =
    println(s"${same(1, 1)} ${pairs((1, 2))} ${alike(2, 3)} ${pairsAlike((1, 1))} $shown $wide")
