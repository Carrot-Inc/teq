// expect: 13:33: error: ambiguous given instances for Int: value, given_Int
// expect: 1 error found
// An inline body's import and given of one block stand in one scope where the body expands, as
// they do in the block: an implicit `Int` imported by a wildcard and a local `given Int` are
// ambiguous, as scalac finds them.
import scala.compiletime.summonInline
object Values:
  implicit val value: Int = 7
inline def f: Int =
  import Values.*
  given Int = 8
  summonInline[Int]
@main def run(): Unit = println(f)
