// A typed pattern of a class on a scrutinee whose class is a trait of the library's (and the reverse) is no
// error: a subclass may be both, so dotty's type test (`TypeTestsCasts.checkSensical`) lets it through and the
// space check alone says the case is unreachable. The std's `Map` and `Iterator` are traits as scala-library's.
def kind(m: Map[String, Int]): String = m match
  case l: List[?] => "a list"
  case _ => "a map of " + m.size
def first(xs: List[Int]): String = xs match
  case it: Iterator[?] => "an iterator"
  case _ => "a list of " + xs.length
@main def run() =
  println(kind(Map("a" -> 1)))
  println(first(List(1, 2)))
