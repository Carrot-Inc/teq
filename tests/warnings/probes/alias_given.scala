// A given an import of a stable value's member makes, taken by the implicit search.
class C { given n: Int = 1 }
object Use {
  def f(c: C): Int = {
    import c.n
    summon[Int]
  }
}
