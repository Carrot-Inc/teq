// A collection where a function of one argument is expected, as scala-library declares them:
// a `Set[A]` is an `A => Boolean`, a `Map[K, V]` a `K => V` and a `Seq[A]` an `Int => A`.
package collectionasfunction

object Main:
  def count(xs: List[Int], f: Int => Boolean): Int = xs.count(f)
  def lookup(f: String => Int): Int = f("b")

  def main(args: Array[String]): Unit =
    val vowels = Set('a', 'e', 'i', 'o', 'u')
    println(s"${"education".filter(vowels)} ${"sky".exists(vowels)} ${List("a", "b").map(Set("a"))}")
    val m = Map(1 -> "one", 2 -> "two")
    println(s"${List(2, 1).map(m)} ${List(1, 2, 3).indexWhere(Set(2, 3))} ${lookup(Map("a" -> 1, "b" -> 2))}")
    val names = Vector("x", "y", "z")
    println(s"${List(2, 0).map(names)} ${(0 until 3).map(names).mkString}")
    val p: Int => Boolean = Set(1, 2)
    println(s"${p(1)} ${p(3)} ${count(List(1, 2, 3, 4), Set(2, 4))}")
    var built = 0
    def made(): Set[Int] =
      built += 1
      Set(3)
    println(s"${List(1, 2, 3, 3).filter(made())} $built")
