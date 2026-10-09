package pl

import scala.compiletime.summonInline

// 𝄞𝄢 supplementary characters before the bodies: their UTF-16 and byte offsets differ
object Places:
  def direct(b: PlBox): Int = "𝄞".length + b.removed
  def nested(b: PlBox): Int = Helpers.removedOf(b)
  def one: Int = Helpers.sharedRemoved
  def two: Int = Helpers.sharedRemoved + 1
  def applied(b: PlBox): Int = b.plus(1)
  def operator(b: PlBox): Int = b ++ 1
  def fine(b: PlBox): Int = b.v
  inline def less[T](x: T, y: T): Boolean = summonInline[Ordering[T]].lt(x, y)
