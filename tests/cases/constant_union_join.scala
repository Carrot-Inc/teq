// Two constants join to their union, soft (dotty's `lub`: disjoint atoms make an `OrType`),
// which an inferred type widens to their class and an expected `1 | 2` keeps; the union has its
// class's members and operators (`OrType.join`).
def test(c: Boolean) =
  val xs: List[1 | 2] = List(if c then 1 else 2)
  xs
@main def run() =
  println(test(true))
  val v = if test(false).isEmpty then 1 else 2
  val w: Int = v
  println(w + 10)
  println((if w > 0 then 1 else 2).toDouble + 0.5)
  println(10 + (if w > 0 then 1 else 2))
  println((if w > 0 then true else false) && w > 1)
  println((if w > 0 then 0 else 97) == 0L)
  println((if w > 0 then "a" else "b") == "a")
