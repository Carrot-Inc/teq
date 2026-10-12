// `lazyZip` reads the pairs off the two collections' iterators as an operation asks for them, as
// scala-library's `LazyZipOps` does: `exists` stops at its answer, `forall` at the first failure, and
// `map` and `filter` build from a view, so a lazy collection's elements past what is needed are
// never computed.
@main def run(): Unit =
  var count = 0
  val xs = LazyList(1, 2, 3).map { x => count += 1; x }
  println(xs.lazyZip(List(9)).exists((a, b) => a == 1))
  println(count)
  count = 0
  val ys = LazyList(1, 2, 3, 4).map { x => count += 1; x }
  println(ys.lazyZip(List(1, 2, 0, 4)).forall((a, b) => a == b))
  println(count)
  println(List(1, 2, 3).lazyZip(List("a", "b")).map((n, s) => s * n))
  println(List(1, 2, 3, 4).lazyZip(List(4, 3, 2, 1)).filter((a, b) => a < b))
  println(Vector(1, 2).lazyZip(List(10, 20)).flatMap((a, b) => List(a, b)))
  var seen = 0
  List(1, 2, 3).lazyZip(List(1, 1, 1)).foreach((a, b) => seen += a * b)
  println(seen)
