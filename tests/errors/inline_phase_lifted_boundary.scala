// expect: 11:19: error: type mismatch: found 1, required String
// expect: 1 error found
// A plain call in a class of a block waits for the inlining phase as every plain call does, which
// no program the typer reports an error in reaches: its `compiletime.error` is not reported.
import scala.compiletime.error
inline def boom: Int = error("EXPANDED")
def f: Int =
  class Local:
    val x = boom
  new Local().x
val bad: String = 1
