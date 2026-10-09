package fix.rassoc

class Vec(val xs: List[Int])

object RightAssoc:
  extension (v: Vec)
    def +:(x: Int): Vec = new Vec(x :: v.xs)
    def :+(x: Int): Vec = new Vec(v.xs :+ x)
  extension [A](v: Vec)(using o: Ordering[A])
    def ++:(as: List[A]): Vec = v
  // A default of a later clause, whose getter takes the clauses before it in the declaration's
  // order: the method's own first clause, then the receiver's.
  extension (r: Int)
    def +::(l: Int)(d: Int = r * 10 + l): Int = d
