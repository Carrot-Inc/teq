// A typed pattern between one of `List`, `Option`, `Either`, `Try` (classes) and `mutable.Set`, a trait of scala-library's:
// scalac warns that the case is unreachable and the program runs (tests/cases/class_pattern_on_library_trait).
// The std declares the name as an alias of the class `HashSet`, so teq refuses the pattern as one between two unrelated classes.
import scala.annotation.nowarn
import scala.collection.mutable

@nowarn def onOption(o: Option[Int]): String = o match
  case s: mutable.Set[?] => "a set"
  case _ => "an option of " + o.size
@nowarn def onSet(s: mutable.Set[Int]): String = s match
  case l: List[?] => "a list"
  case _ => "a set of " + s.size
@main def run() =
  println(onOption(Some(1)))
  println(onSet(mutable.Set(1, 2)))
