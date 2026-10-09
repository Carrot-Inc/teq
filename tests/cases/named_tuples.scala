// Named tuples (Scala 3.7): the type and literal syntax, selection by name, `toTuple`,
// patterns with names in any order and as a subset, and the conformance between a named tuple
// and the plain tuple it erases to.
case class Person(name: String, age: Int)

object Main:
  type Ping = (seq: Int, sentAt: String)
  case class Order(id: Int, ping: Option[Ping])
  case class Batch(pings: Map[Int, (seq: Int, sentAt: String)])

  def describe(p: Ping): String = s"${p.seq}@${p.sentAt}"
  def latest(pings: List[Ping]): Option[Int] = pings.map(_.seq).maxOption

  def main(args: Array[String]): Unit =
    val p: Ping = (seq = 1, sentAt = "t")
    println(p)
    println(p.seq + p.sentAt)
    println(p.toTuple)
    println(p.toTuple._2)
    val plain: (Int, String) = (2, "u")
    val fromPlain: Ping = plain
    println(describe(fromPlain))
    println(describe((3, "v")))
    val back: (Int, String) = p
    println(back._1)
    val m: Map[Int, Ping] = Map(1 -> (seq = 4, sentAt = "w"), 2 -> (5, "x"))
    println(m(2).seq)
    println(Batch(Map(7 -> (8, "y"))).pings(7).sentAt)
    p match
      case (seq = i, sentAt = t) => println(s"both $i $t")
    p match
      case (sentAt = t) => println(s"time $t")
    p match
      case (sentAt = t, seq = i) => println(s"reordered $i $t")
    p match
      case (a, b) => println(s"positional $a $b")
    val (x, y) = p
    println(x + y)
    println(p == (1, "t"))
    println(p == (seq = 1, sentAt = "t"))
    println(p == (seq = 2, sentAt = "t"))
    val inferred = (name = "n", count = 2)
    println(inferred.count + inferred.name.length)
    println(Order(1, Some((seq = 9, sentAt = "z"))).ping.map(_.seq))
    val nested: (outer: (inner: Int, other: String), flag: Boolean) = (outer = (inner = 1, other = "o"), flag = true)
    println(nested.outer.inner)
    println(latest(List(p, (7, "q"), (seq = 6, sentAt = "r"))))
    val f: Ping => String = q => q.sentAt * q.seq
    println(f((seq = 3, sentAt = "ab")))
    Person("ann", 41) match
      case Person(age = a, name = n) => println(s"$n is $a")
    Person("bob", 7) match
      case Person(name = "bob") => println("named bob")
      case _ => println("someone else")
    val people = List(Person("c", 1), Person("d", 2)).collect { case Person(age = a) if a > 1 => a }
    println(people)
