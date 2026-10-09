package cst

// Aliases and constants: the alias as written and what it stands for, a constant's type.
object Limits:
  type Count = Int
  type Named[A] = Map[String, A]
  final val Max = 10
  final val Label = "limit"
  inline val Twice = 2
  val three: 3 = 3
  def cap(n: Count): Count = if n > Max then Max else n
  def table: Named[Count] = Map(Label -> Max * Twice)
  val ratio: Double = Max / 4.0

class User:
  def check(n: Limits.Count): Boolean = n == Limits.Max
  val pair: (Limits.Count, String) = (Limits.three, Limits.Label)
