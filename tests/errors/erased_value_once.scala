// expect: 9:11: error: method erasedValue is declared as `erased`, but is in fact used
// expect: 1 error found
// absent: 10:11: error
// Two expansions of one inline body keep its `erasedValue` as a value: scalac's
// `Erasure.checkNotErased` reports each at its call, and `UniqueMessagePositions` hides the second,
// whose inlined place the first took; the vals' types inferred, so that the expansions come late.
import scala.compiletime.erasedValue
inline def bad[T]: T = erasedValue[T]
val one = bad[Int]
val two = bad[String]
