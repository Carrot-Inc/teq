package mta

type Elem[X] = X match
  case String => Char
  case Int => Long

object Elems:
  def first[X](x: X): Elem[X] = (x match
    case s: String => s.charAt(0)
    case i: Int => i.toLong
  ).asInstanceOf[Elem[X]]
