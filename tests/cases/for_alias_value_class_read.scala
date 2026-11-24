// A for comprehension's variable of a value class, bound by an alias and read by what follows it (a guard, a yield,
// a second generator, a nested for, `foreach`). Before a guard the comprehension carries the alias's variables to the
// guard's closure in one pack and reads each back at its own type, a value class's as its box; the other readers keep
// their own reads.
final class Meters(val u: Long) extends AnyVal
final class Count(val u: Int) extends AnyVal
final class Ratio(val u: Double) extends AnyVal
final class Name(val u: String) extends AnyVal

object Main {
  def main(args: Array[String]): Unit = {
    println(for { k <- List(1, 2); (n, v) = (k, new Meters(7L)); if n > 1 } yield v.u)
    println(for { k <- List(1, 2); (n, v) = (k, new Meters(k * 3L)) } yield n + v.u)
    println(for { k <- List(1, 2); v = new Count(k * 2); if v.u > 2 } yield v.u)
    println(for { k <- List(1, 2); (_, v) = (k, new Ratio(k * 1.25)); m <- List(10, 20) } yield v.u + m)
    println(for { k <- Option(3); (n, v) = (k, new Name("n" + k)) } yield (for { m <- List(1) } yield v.u + n + m))
    println(for { k <- 1 to 2; v = new Meters(k * 5L); if v.u > 5 } yield v.u)
    for { k <- List(1, 2); (n, v) = (k, new Name("s" + k)) } println(v.u + n)
  }
}
