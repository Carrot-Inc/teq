package isp

// A given found inside another's search, the argument of `listHead` at each of two calls: dotty
// places the arguments a search inserts at the call that asked for them (`Typer.adaptNoArgs`'s
// `implicitArgs`), each its own, whatever search found the same given before.
trait Codec[A]
object Codec:
  given instant: Codec[java.time.Instant] = new Codec[java.time.Instant] {}
  given listHead[A](using c: Codec[A]): Codec[List[A]] = new Codec[List[A]] {}

def query[A](name: String)(using c: Codec[List[A]]): String = name

// A search of the same type in this module first: a whole build keeps its result for the next.
object Warm:
  val first = query[java.time.Instant]("first")
