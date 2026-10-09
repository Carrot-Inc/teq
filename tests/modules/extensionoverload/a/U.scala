package eoa

case class Day(n: Int)
case class Hour(n: Int)

object U:
  extension (t: Hour)
    def fmt: String = "hour " + t.n
  extension (d: Day)
    def fmt: String = "day " + d.n
