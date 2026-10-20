// lila: ForumPostApi.scala:211/216, RatingChartApi.scala:35/42, ClasApi.scala:280/285
// A `.value` line outdented below an indented `for ... yield` body selects on that whole body.
case class OptionT[A](value: Option[A]):
  def map[B](f: A => B): OptionT[B] = OptionT(value.map(f))
  def flatMap[B](f: A => OptionT[B]): OptionT[B] = OptionT(value.flatMap(a => f(a).value))

object Api:
  def find(n: Int): Option[Int] = Option.when(n > 0)(n)

  def build(a: Int, b: Int): Option[Int] =
    for
      x <- OptionT(find(a))
      y <- OptionT(find(b))
    yield x + y
  .value

@main def run() =
  println(Api.build(1, 2))
  println(Api.build(1, 0))
