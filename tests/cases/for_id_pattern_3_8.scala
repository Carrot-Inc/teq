// teq: --source 3.8 --wconf any:s
// `makeLambda`'s `IdPattern` (the review of 1888ac4df): an identifier, a typed one (`_: T` among them) is the
// function's parameter, `(x)` and `(K)` a case lambda's pattern; a literal or a stable reference as a pattern has
// its singleton type (`1 <- List[1](1)`, `(O) <- List(O)` irrefutable); a tuple under N parameters takes direct
// parameters (`patternsToParams`) or the tuple of `x$1`..`x$N` matched, checked as a pattern definition where the
// generator's selector was unchecked; an `@unchecked` selector silences its own type tests alone.
object O
final val F = 2
class Z(xs: List[(Int, Option[Int])]):
  def withFilter(p: ((Int, Option[Int])) => Boolean): Z2 = Z2(xs.filter(p))
class Z2(xs: List[(Int, Option[Int])]):
  def foreach(f: (Int, Option[Int]) => Unit): Unit = xs.foreach((a, b) => f(a, b))
@main def Test =
  for (O) <- List(O) do println("o")
  for 1 <- List[1](1) do println("one")
  for (F) <- List[2](2) do println("f")
  for (x) <- List(3) do println(x)
  for X <- List(4) do println(X)
  for `y` <- List(5) do println(y)
  for _: Int <- List(6) do println("six")
  for x: Any <- List(7) do println(x)
  for (x: Int) <- List(8) do println(x)
  val xs = List(1, 2); val ys = List(3, 4)
  for (A, b) <- xs.lazyZip(ys) do println(A + b)
  for (a: Int, b) <- xs.lazyZip(ys) do println(a * b)
  for (a, (b)) <- xs.lazyZip(ys) do println(a - b)
  for (_, b) <- xs.lazyZip(ys) do println(b)
  for case (a, Some(b)) <- Z(List((1, Some(2)), (3, None))) do println(a + b)
  val as: List[Any] = List(List(1))
  for case x: List[Int] <- as do println(x)
  for case (y: List[Int], z) <- List[(Any, Int)]((List(1), 2)) do println(y)
  for case w @ ((_: List[Int]) | (_: Set[Int])) <- as do println(w)
