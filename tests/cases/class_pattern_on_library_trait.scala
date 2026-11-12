// A typed pattern of a class on a scrutinee whose class is a trait of the library's, and the reverse: scalac
// warns that the case is unreachable (tests/warnings/matches/class_pattern_on_library_trait) and the program
// runs. With `List` a class and the std's `Map` and `Iterator` classes too, teq refused it as two unrelated
// classes.
import scala.annotation.nowarn

@nowarn def kind(m: Map[String, Int]): String = m match
  case l: List[?] => "a list"
  case _ => "a map of " + m.size
@nowarn def first(xs: List[Int]): String = xs match
  case it: Iterator[?] => "an iterator"
  case _ => "a list of " + xs.length
@main def run() =
  println(kind(Map("a" -> 1)))
  println(first(List(1, 2)))
