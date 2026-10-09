package demandedgiven

import scala.compiletime.summonInline

object Holder:
  given own: String = "own"
  given count: Int = 7

  inline def pick: Int = summonInline[Int]

  def value = summonInline[String]
  def number = summonInline[Int]
  def nested = pick
