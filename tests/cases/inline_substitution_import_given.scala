// A block import of an inline body is in scope where the body expands: `summonInline[Int]` finds
// the given `import Values.given` brings in, 7, as scalac 3.8.4 finds it; the expansion by
// substitution reads the import from the stored body's record.
import scala.compiletime.summonInline
object Values:
  given Int = 7
inline def f: Int =
  import Values.given
  summonInline[Int]
@main def run(): Unit = println(f)
