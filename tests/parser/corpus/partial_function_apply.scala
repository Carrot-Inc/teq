// A partial function literal applied directly and through the collections.
package partialfunctionapply

@main def main(): Unit =
  val pf: PartialFunction[Int, Int] = { case x if x > 1 => x * 10 }
  println(pf.isDefinedAt(2))
  println(pf(2))
  println(pf.apply(3))
  println(List(1, 2, 3).collectFirst(pf))
  println(List(1, 2, 3).collect(pf))
  println(pf.lift(0))
