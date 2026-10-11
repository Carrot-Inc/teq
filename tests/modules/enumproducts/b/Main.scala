package enumproducts

object Main:
  def main(args: Array[String]): Unit =
    println(E.A.n)
    println(E.A.v)
    println(E.A.f)
    val a = E.values
    a(0) = E.B
    println(E.values.toList)
    println(M.fromOrdinal(0))
    try println(M.fromOrdinal(1)) catch case _: NoSuchElementException => println("missing")
