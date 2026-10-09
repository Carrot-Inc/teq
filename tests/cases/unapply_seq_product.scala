// `unapplySeq` returning a product whose last element is the sequence: the fields before it
// match their patterns, the sequence the patterns that remain.
class Node(val name: String, val children: Int*)
object Node:
  def unapplySeq(n: Node): Option[(String, Seq[Int])] = Some((n.name, n.children))
object Words:
  def unapplySeq(s: String): Option[(Int, Seq[String])] =
    val ws = s.split(" ").toSeq
    Some((ws.length, ws))

object Main:
  def describe(n: Node): String = n match
    case Node(name, a, b, rest*) => s"$name: $a, $b and ${rest.length} more"
    case Node(name, one) => s"$name: only $one"
    case Node(name) => s"$name: none"
    case Node(name, all*) => s"$name: ${all.sum}"
  def main(args: Array[String]): Unit =
    println(describe(new Node("a", 1, 2, 3, 4)))
    println(describe(new Node("b", 7)))
    println(describe(new Node("c")))
    println(describe(new Node("d", 1, 2)))
    "one two three" match
      case Words(n, first, rest*) => println(s"$n words, first $first, then ${rest.mkString("+")}")
    "solo" match
      case Words(1, w) => println(s"just $w")
      case _ => println("more")
