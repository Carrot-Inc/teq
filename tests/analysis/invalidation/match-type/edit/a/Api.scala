package mta

type Elem[X] = X match
  case String => Int
  case Int => Long

object Elems:
  def first[X](x: X): Elem[X] = (x match
    case s: String => s.length
    case i: Int => i.toLong
  ).asInstanceOf[Elem[X]]
