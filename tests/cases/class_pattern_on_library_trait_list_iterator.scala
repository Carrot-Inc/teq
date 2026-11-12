// A typed pattern between one of `List`, `Option`, `Either`, `Try` (classes) and `java.util.ListIterator`, an
// interface of the JDK's: scalac warns that the case is unreachable and the program runs
// (tests/cases/class_pattern_on_library_trait). The std declares it as a class, so teq refuses the pattern as one
// between two unrelated classes.
import scala.annotation.nowarn
import java.util.ListIterator

@nowarn def onList(xs: List[Int]): String = xs match
  case it: ListIterator[?] => "a list iterator"
  case _ => "a list of " + xs.length
@nowarn def onIterator(it: ListIterator[Int]): String = it match
  case o: Option[?] => "an option"
  case _ => "an iterator"
@main def run() =
  println(onList(List(1, 2)))
