package p02
object Lambdas:
  val f = (x: Int) => x + 1
  val g: Int => Int = _ * 2
  val h = List(1, 2).map(_ + 1)
  val k = List(1, 2).foldLeft(0)(_ + _)
  val m = List(1, 2).map { y => val z = y; z }
  val ctx: Int ?=> Int = summon[Int]
  val pf: PartialFunction[Int, Int] = { case n if n > 0 => n }
  val eta = List(1).map(inc)
  def inc(i: Int) = i + 1
  val two = (a: Int, b: Int) => a + b
  val poly = [T] => (t: T) => t
