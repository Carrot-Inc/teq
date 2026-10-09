package aua

// Anonymous using parameters of classes and traits: scalac names each `x$N`, N its place among
// every term parameter before it and its own, the constructor's parameter, the accessor and the
// body's selection of it alike, by which a downstream's class implements a trait's.
trait T(val tag: String)(using Int):
  def answer: Int = summon[Int]

trait Two(val k: Int)(using String, Int):
  def both: String = s"$k ${summon[String]}${summon[Int]}"

class Mixed(x: Int)(using Boolean)(y: Int)(using Char, Long):
  def flags: String = s"$x ${summon[Boolean]} $y ${summon[Char]} ${summon[Long]}"

trait Wide[A](first: A, second: A)(using Ordering[A]):
  def larger: A = summon[Ordering[A]].max(first, second)

def twice(n: Int)(using Int): Int = n * summon[Int]

extension (s: String)
  def times(n: Int)(using Char): String = List.fill(n)(s).mkString + summon[Char]

given label(using Int): String = "n" + summon[Int]
