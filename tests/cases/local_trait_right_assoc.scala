// A local trait's right-associative extension member, a captured trait's (its members take the block's values first)
// and an uncaptured one's: one and two argument lists, the left operand a literal, a path, a call and a block, called
// through an import of the instance and from the trait itself.
object Main {
  var log = List.empty[String]
  def note(s: String): String = { log = log :+ s; s }
  def num(n: Int): Int = { log = log :+ ("n" + n); n }

  def captured(prefix: String): String = {
    trait Local {
      extension (n: Int) def +:(s: String): String = prefix + s + n
      extension (n: Int) def +::(s: String)(k: Int): String = prefix + s + n + "/" + k
      def inside(m: Int): String = num(m) +: note("i")
    }
    val t = new Local {}
    import t.*
    val v = 7
    val a = 3 +: "x"
    val b = v +: note("p")
    val c = num(5) +: note("c")
    val d = { log = log :+ "blk"; 4 } +: note("b")
    val e = (num(6) +:: note("two"))(9)
    val f = t.inside(8)
    List(a, b, c, d, e, f).mkString(",")
  }

  def uncaptured(): String = {
    trait Plain {
      extension (n: Int) def +:(s: String): String = "q" + s + n
    }
    val t = new Plain {}
    import t.*
    val v = 2
    List(1 +: "x", v +: note("u"), num(3) +: note("v"), { log = log :+ "ublk"; 4 } +: note("w")).mkString(",")
  }

  def main(args: Array[String]): Unit = {
    println(captured("p"))
    println(uncaptured())
    println(log.mkString(","))
  }
}
