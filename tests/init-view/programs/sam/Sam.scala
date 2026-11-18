abstract class F { def ap(i: Int): Int }

class S {
  val pf: PartialFunction[Int, Int] = { case n => n + k }
  val r = pf(1)
  val f: F = i => i + k
  val q = f.ap(1)
  val k = 4
}
