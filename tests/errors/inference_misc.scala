// Expected types through Some, tuples, lambdas and folds.
// expect: 18:12: error: method paramless in object M does not take parameters
object M:
  val c = true
  val w5: List[? <: AnyVal] = List("a")          // scalac: ?
  val nested: List[Int => Int] = List(x => x + 1, x => x * 2)
  def opt(x: Option[Long]): Long = x.getOrElse(0L)
  val o1 = opt(Some(1))
  val o2 = opt(if c then Some(1) else None)
  def pair(p: (String, Long)): Long = p._2
  val p1 = pair(("a", 1))
  val p2 = pair("a" -> 1)
  val m1: Map[String, Double] = Map("a" -> 1, "b" -> 2)
  val v1: Vector[Long] = Vector(1, 2)
  val s1: Set[Double] = Set(1, 2)
  val nullaryEta: () => Int = () => 1
  def paramless: Int = 2
  val g4 = paramless()                           // scalac: error
  val fold = List(1, 2).foldLeft(Nil)((acc, x) => x :: acc)   // accepted, as scalac accepts it
  val foldt: List[Int] = fold
  def sink[A](fn: A => Unit): A => Unit = fn
  val n1 = sink(_.toString)                      // scalac: A := Any (flipBottom)
  val n1t: Any => Unit = n1

@main def main(): Unit =
  println(M.o1)
  println(M.p2)
  println(M.m1)
