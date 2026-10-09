// The evidence of context bounds joins a trailing using clause, in front of its parameters.
trait Show[A] { def n: String }
trait Ord[A] { def m: String }
class S(val n: String) extends Show[Int]
class O(val m: String) extends Ord[Int]
given Show[Int] = S("showInt")
given Ord[Int] = O("ordInt")

def k[A: Show, B: Ord](a: A, b: B)(using extra: Ord[A]) = summon[Show[A]].n + summon[Ord[B]].m + extra.m
def k2[A: Show](a: A)(using extra: Ord[A])(b: Int) = summon[Show[A]].n + extra.m + b
def k3[A: Show](a: A) = summon[Show[A]].n

@main def run(): Unit =
  println(k(1, 2))
  println(k(1, 2)(using S("explicitShow"), O("explicitOrdB"), O("explicitExtra")))
  println(k2(1)(using O("o"))(3)(using S("s")))
  println(k3(1)(using S("s3")))
