// A typed pattern between one of `List`, `Option`, `Either`, `Try` (classes) and `mutable.SortedSet`, a trait of scala-library's:
// scalac warns that the case is unreachable and the program runs (tests/cases/class_pattern_on_library_trait).
// teq is to run the program too.
import scala.annotation.nowarn
import scala.collection.mutable
import scala.util.Try

@nowarn def onList(xs: List[Int]): String = xs match
  case s: mutable.SortedSet[?] => "a sorted set"
  case _ => "a list of " + xs.length
@nowarn def onSet(s: mutable.SortedSet[Int]): String = s match
  case t: Try[?] => "a try"
  case _ => "a sorted set of " + s.size
@main def run() =
  println(onList(List(1, 2)))
  println(onSet(mutable.SortedSet(3, 1, 2)))
