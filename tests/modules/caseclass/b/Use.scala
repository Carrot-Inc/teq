package cb

import ca.*

object Use:
  def describe(p: Person): String = p match
    case Person(n, a) if a > 18 => n + " adult"
    case Person(n, _) => n
  def main(args: Array[String]): Unit =
    val p = Person("ann", 20)
    println(describe(p.copy(age = 3)))
    println(Box(1, Nil).copy(value = "s").value)
    val Pair(l, r) = Pair(1, 2)
    println(l + r + Pair(3, 4).sum)
    println(Person("bob"))
