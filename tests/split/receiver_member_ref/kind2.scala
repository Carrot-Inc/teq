package rmr

enum Kind2 derives Show:
  case Wide(width: String, depth: Long)
  case Thin(size: Long)
  case Blank

object Uses2:
  def all: String = List(Kind2.Wide("w2", 2), Kind2.Thin(2), Kind2.Blank).map(summon[Show[Kind2]].show).mkString(" ")
