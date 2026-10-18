// teq: --source 3.8
// `Desugar.makeFor.deepEquals` over `unsplice` (the review of 1888ac4df): identifiers by name whatever they denote
// (`case K` and its body `K`), tuples element by element, a body's parentheses unwrapped, a brace block none.
class Seq1[A](val xs: List[A]):
  def map[B](f: A => B): Seq1[B] = { println("map"); Seq1(xs.map(f)) }
  def withFilter(p: A => Boolean): Seq1[A] = Seq1(xs.filter(p))
  def flatMap[B](f: A => Seq1[B]): Seq1[B] = Seq1(xs.flatMap(a => f(a).xs))
  override def toString = xs.mkString("Seq1(", ", ", ")")
val K = 1
@main def Test =
  val s = Seq1(List(1, 2))
  println(for case K <- s yield K)
  println(for (a, b) <- Seq1(List((1, 2))) yield (a, b))
  println(for (a, (b, c)) <- Seq1(List((1, (2, 3)))) yield (a, (b, c)))
  println(for x <- s yield (x))
  println(for x <- s yield { x })
  println(for (a, b) <- Seq1(List((1, 2))) yield (b, a))
  println(for x <- s yield ((x)))
