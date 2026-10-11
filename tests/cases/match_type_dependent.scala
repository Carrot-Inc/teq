// A match whose cases have the expected match type's shape is typed case by case against the
// match type's bodies (dotty's `Typer.typedDependentMatchFinish`, gated by `isMatchTypeShaped`):
// as many cases, no guard, typed patterns equal to the case patterns with the pattern's own type
// variables for the binders, a wildcard for a case of `Any`.
type Elem[X] = X match
  case String => Char
  case Array[t] => t
  case List[t] => t

def head[X](x: X): Elem[X] = x match
  case s: String => s.charAt(0)
  case a: Array[t] => a(0)
  case l: List[t] => l.head

type Wrapped[X] = X match
  case Int => String
  case _ => X

def wrap[X](x: X): Wrapped[X] = x match
  case i: Int => s"<$i>"
  case _ => x

@main def run() =
  println(head("abc"))
  println(head(Array(7, 8)))
  println(head(List("x", "y")))
  val c: Char = head("zq")
  println(c)
  println(wrap(3))
  println(wrap("s"))
