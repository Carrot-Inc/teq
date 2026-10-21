// A placeholder's parameter (`WildcardParamName`, a derived name): module a parsed alone (one thread) and in the
// whole build, which the generated sibling makes large enough to be parsed in parallel (`frontend::parse_files`), pickles
// it as one name, `UNIQUE _$ 0` (the gate of `ed4dafbea`'s app chain).
package pa

object Ops:
  def inc(xs: List[Int]): List[Int] = xs.map(_ + 1)
  inline def twice(xs: List[Int]): List[Int] = xs.map(_ * 2)
  def both(f: (Int, Int) => Int): Int = f(1, 2)
  def sum: Int = both(_ + _)
