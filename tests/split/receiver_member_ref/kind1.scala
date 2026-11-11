package rmr

enum Kind1 derives Show:
  case Wide(width: String, depth: Long)
  case Thin(size: Long)
  case Blank

object Uses1:
  def all: String = List(Kind1.Wide("w1", 1), Kind1.Thin(1), Kind1.Blank).map(summon[Show[Kind1]].show).mkString(" ")
