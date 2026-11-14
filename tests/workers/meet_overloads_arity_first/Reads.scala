// The first part of the intersections of Use.scala: two overloads of `at`, the second naming a val of
// inferred type.
trait Reads:
  def at(key: String): String
  def at(slot: Stock.first.Slot): Int
