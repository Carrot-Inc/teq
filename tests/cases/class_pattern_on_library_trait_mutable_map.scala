// A typed pattern between one of `List`, `Option`, `Either`, `Try` (classes) and `mutable.Map`, a trait of scala-library's:
// scalac warns that the case is unreachable and the program runs (tests/cases/class_pattern_on_library_trait).
// The std declares the name as an alias of the class `HashMap`, so teq refuses the pattern as one between two unrelated classes.
import scala.annotation.nowarn
import scala.collection.mutable

@nowarn def onList(xs: List[Int]): String = xs match
  case m: mutable.Map[?, ?] => "a map"
  case _ => "a list of " + xs.length
@nowarn def onMap(m: mutable.Map[String, Int]): String = m match
  case l: List[?] => "a list"
  case _ => "a map of " + m.size
@main def run() =
  println(onList(List(1, 2)))
  println(onMap(mutable.Map("a" -> 1)))
