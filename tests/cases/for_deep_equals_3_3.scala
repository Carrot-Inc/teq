// teq: --source 3.3
// Before better fors (3.3) no trailing `map` is left out (the review of 1888ac4df).
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
