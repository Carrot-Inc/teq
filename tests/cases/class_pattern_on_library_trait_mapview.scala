// A typed pattern between one of `List`, `Option`, `Either`, `Try` (classes) and `MapView`, a trait of scala-library's:
// scalac warns that the case is unreachable and the program runs (tests/cases/class_pattern_on_library_trait).
// The std declares it as a class, so teq refuses the pattern as one between two unrelated classes.
import scala.annotation.nowarn
import scala.collection.MapView

@nowarn def onList(xs: List[Int]): String = xs match
  case v: MapView[?, ?] => "a view"
  case _ => "a list of " + xs.length
@nowarn def onView(v: MapView[String, Int]): String = v match
  case o: Option[?] => "an option"
  case _ => "a view of " + v.size
@main def run() =
  println(onList(List(1, 2)))
  println(onView(Map("a" -> 1).view))
