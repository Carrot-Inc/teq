// expect: too many arguments: expected 1
trait Show[A] { def n: String }
trait Ord[A] { def m: String }
class S(val n: String) extends Show[Int]
class O(val m: String) extends Ord[Int]
given Show[Int] = S("showInt")
given Ord[Int] = O("ordInt")

// The evidence is appended as a clause of its own when the last clause is not a using clause.
def k2[A: Show](a: A)(using extra: Ord[A])(b: Int) = summon[Show[A]].n + extra.m + b

@main def run(): Unit =
  println(k2(1)(using S("s"), O("o"))(3))
