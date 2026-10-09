//> using scala 3.8.4
// The overloads of the standard library that teq's has as well: `mkString` with no, one and
// three arguments as members, and the deprecated vararg forms of `+` and `+=`, which
// `set + (a, b)` means in Scala.
import scala.collection.mutable

@main def run(): Unit =
  val titles = List("Dune", "Emma", "Ulysses")
  println(titles.mkString)
  println(titles.mkString(", "))
  println(titles.mkString("[", "; ", "]"))
  println(titles.iterator.mkString("/"))
  println(Vector(1, 2).mkString + Set(3).mkString("-") + Map(1 -> "a").mkString("{", ",", "}"))
  val join: String => String = titles.mkString
  println(join(" & "))
  println(Set(1) + (2, 3))
  val shelf = mutable.ArrayBuffer(1)
  shelf += (2, 3)
  shelf += 4
  println(shelf)
  println(Map(1 -> "a") + (2 -> "b", 3 -> "c"))
  println(Set("x") + "y")
  println(Map(1 -> "a") + (2 -> "b"))
