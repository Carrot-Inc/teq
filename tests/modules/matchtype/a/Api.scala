package mta

type Elem[X] = X match
  case String => Char
  case List[t] => t

object Elems:
  def first[X](x: X): Elem[X] = (x match
    case s: String => s.charAt(0)
    case xs: List[?] => xs.head
  ).asInstanceOf[Elem[X]]
