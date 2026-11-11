package rmr

enum Kind3 derives Show:
  case Wide(width: String, depth: Long)
  case Thin(size: Long)
  case Blank

object Uses3:
  def all: String = List(Kind3.Wide("w3", 3), Kind3.Thin(3), Kind3.Blank).map(summon[Show[Kind3]].show).mkString(" ")
