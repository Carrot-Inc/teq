// A case object nested in a class is one per enclosing instance: two of them are unequal,
// hash alike, print alike and are two elements of a set; a match on a stable path to one takes
// that path's instance alone.
class O(val tag: String):
  case object R
  object Plain

def label(x: Any, a: O, b: O): String = x match
  case a.R => "a"
  case b.R => "b"
  case _ => "other"

@main def run(): Unit =
  val a = O("a")
  val b = O("b")
  println(a.R == b.R)
  println(a.R eq b.R)
  println(a.R == a.R)
  println(a.R.## == b.R.##)
  println(Set[Any](a.R, b.R).size)
  println(a.R.toString + b.R)
  println(a.Plain == b.Plain)
  println(label(b.R, a, b) + label(a.R, a, b) + label(1, a, b))
