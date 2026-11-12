// A typed pattern between one of `List`, `Option`, `Either`, `Try` (classes) and `java.nio.file.Path`, an interface of the JDK's:
// scalac warns that the case is unreachable and the program runs (tests/cases/class_pattern_on_library_trait).
// The std declares it as a class, so teq refuses the pattern as one between two unrelated classes.
import scala.annotation.nowarn
import java.nio.file.{Path, Paths}

@nowarn def onOption(o: Option[Int]): String = o match
  case p: Path => "a path"
  case _ => "an option of " + o.size
@nowarn def onPath(p: Path): String = p match
  case l: List[?] => "a list"
  case _ => "a path to " + p.getFileName
@main def run() =
  println(onOption(Some(1)))
  println(onPath(Paths.get("a", "b")))
