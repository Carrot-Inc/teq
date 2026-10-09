// A lambda of one parameter given for a `PartialFunction` parameter makes the overload
// applicable: with a match as its body the cases decide `isDefinedAt`, whatever the scrutinee;
// otherwise it is defined everywhere (dotc's `ExpandSAMs`). Map's two `collect` overloads both
// take one; the tuple result picks the one that builds a map.
case class P(price: Int)
@main def run(): Unit =
  val m: Map[Int, P] = Map(1 -> P(10), 2 -> P(20), 3 -> P(30))
  println(m.collect(it => it._2.price match { case n if n > 15 => it._1 -> n }))
  println(m.collect(it => it._2.price match { case n if n > 15 => n }))
  println(List(1, 2, 3).collect(x => x match { case 2 => "two" }))
  println(List(1, 2, 3).collect(x => x + 1))
  val pf: PartialFunction[Int, Int] = x => x * 2
  println(pf.isDefinedAt(5))
