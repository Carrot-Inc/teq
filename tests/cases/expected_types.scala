//> using platform js
// Expected types flow into lambdas, blocks, branches, by-name and default arguments, tuples and collections.
object E:
  val c = true
  val f1: Int => Int = x => x + 1
  val f2: Int => Long = x => x                 // ok: body widened to Long
  val f3: Int => Any = x => if c then x else "a"
  val o1: Option[Long] = Some(1)               // ok: literal adapted to Long through expected type
  val o2: Option[Long] = if c then Some(1) else None
  val o3: Option[Double] = c match
    case true => Some(1)
    case false => None
  val l1: List[Long] = List(1, 2, 3)           // ok
  val l2: List[Double] = List(1, 2)            // ok
  def byName(x: => Long): Long = x
  val b1 = byName(1)                           // ok
  val b2 = byName(if c then 1 else 2)          // ok
  def dflt(x: Long = 1, y: Double = 2): Double = x + y
  val d1 = dflt()
  val d2 = dflt(3)                             // ok: 3 adapted to Long
  val blk: Int => Int =
    val k = 2
    x => x * k
  val ifFn: Int => Int = if c then x => x + 1 else x => x - 1
  val matchFn: Int => Int = c match
    case true => x => x + 1
    case false => x => x - 1
  val tup: (Long, Double) = (1, 2)             // ok
  val tup2: (Int => Int, String) = (x => x + 1, "a")
  val nested: List[Int => Int] = List(x => x + 1, x => x * 2)
  val map1: Map[String, Long] = Map("a" -> 1)  // ok? ArrowAssoc gives (String, Int); Int not <: Long
  val map2: Map[String, Long] = Map(("a", 1))  // ok? tuple literal adapted
  def taking(fn: Int => Int): Int = fn(1)
  val t1 = taking(_ + 1)
  val t2 = taking { x => x + 1 }
  val t3 = taking:
    x => x + 1
  def poly[A](fn: A => A)(a: A): A = fn(a)
  val p1 = poly[Int](_ + 1)(1)
  val bigIf: Long = if c then 1 else 2
  val seqFn: Seq[Int] => Int = _.size
  val partial: PartialFunction[Int, String] = { case 1 => "one" }

@main def main(): Unit =
  println(E.f2(1))
  println(E.o1)
  println(E.d2)
  println(E.map1)
