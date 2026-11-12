// A typed pattern between one of `List`, `Option`, `Either`, `Try` (classes) and `immutable.SortedMap`, a trait of scala-library's:
// scalac warns that the case is unreachable and the program runs (tests/cases/class_pattern_on_library_trait).
// The std declares it as a class, so teq refuses the pattern as one between two unrelated classes.
import scala.annotation.nowarn
import scala.collection.immutable.SortedMap

@nowarn def onList(xs: List[Int]): String = xs match
  case m: SortedMap[?, ?] => "a sorted map"
  case _ => "a list of " + xs.length
@nowarn def onMap(m: SortedMap[String, Int]): String = m match
  case e: Either[?, ?] => "an either"
  case _ => "a sorted map of " + m.size
@main def run() =
  println(onList(List(1, 2)))
  println(onMap(SortedMap("a" -> 1)))
