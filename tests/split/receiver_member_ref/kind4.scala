package rmr

enum Kind4 derives Show:
  case Wide(width: String, depth: Long)
  case Thin(size: Long)
  case Blank

object Uses4:
  def all: String = List(Kind4.Wide("w4", 4), Kind4.Thin(4), Kind4.Blank).map(summon[Show[Kind4]].show).mkString(" ")
