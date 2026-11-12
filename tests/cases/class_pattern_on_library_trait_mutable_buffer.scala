// A typed pattern between one of `List`, `Option`, `Either`, `Try` (classes) and `mutable.Buffer`, a trait of scala-library's:
// scalac warns that the case is unreachable and the program runs (tests/cases/class_pattern_on_library_trait).
// teq is to run the program too.
import scala.annotation.nowarn
import scala.collection.mutable

@nowarn def onOption(o: Option[Int]): String = o match
  case b: mutable.Buffer[?] => "a buffer"
  case _ => "an option of " + o.size
@nowarn def onBuffer(b: mutable.Buffer[Int]): String = b match
  case o: Option[?] => "an option"
  case _ => "a buffer of " + b.length
@main def run() =
  println(onOption(Some(1)))
  println(onBuffer(mutable.Buffer(1, 2)))
