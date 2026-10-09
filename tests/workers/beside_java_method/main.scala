case class Pair(get: String, other: String) extends java.util.function.Supplier[String]

@main def main(): Unit =
  val p = Pair("a", "b")
  println(p.get + p.other)
