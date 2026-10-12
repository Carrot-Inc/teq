package inlb

import inl.*

@main def run(): Unit =
  val sum: 3 = Defs.sum
  val f: 3f = Defs.f
  val widened: Int = Defs.widened
  val empty: Inv[List[Any]] = Defs.fromEmpty
  val applied: Inv[List[Int]] = Defs.fromApply
  val set: Set[Any] = Defs.fromSet
  val lit: Inv[Int] = Defs.literal
  val contra: Sink[Any] = Defs.contra
  val pair: (Int, String) = Defs.pair
  println((sum, f == 3f, widened, empty.x, applied.x, set, lit.x, pair))
