// `Predef` in source without scala-library's jar: its members are the std's of package `scala`,
// terms, types and the classes of the objects it aliases (`Predef.Map.Map1`).
object Main:
  def main(args: Array[String]): Unit =
    Predef.println("x")
    val s: Predef.String = "s"
    println(Predef.identity(3))
    println(Predef.Map("a" -> 1))
    val m: Predef.Map[String, Int] = Predef.Map.empty
    println(m)
    println(new Predef.Map.Map1("k", 1))
    println(Predef.Set(1))
