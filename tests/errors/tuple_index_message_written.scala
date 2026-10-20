// expect: 10:9: error: index out of bounds: 3
// expect: 11:9: error: SECOND
// expect: 2 errors found
// A program's own `compiletime.error` whose message reads as a tuple index's is the inlining
// phase's error beside the others, as scalac reports it: only the check a definition makes of a
// literal index past a tuple's elements stands for `TupleOptimizations`' later one, told by its mark.
import scala.compiletime.error
inline def a: Int = error("index out of bounds: 3")
inline def b: Int = error("SECOND")
val x = a
val y = b
