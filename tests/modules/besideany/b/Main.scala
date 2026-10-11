package besideany

object Main:
  def main(args: Array[String]): Unit =
    println(D(1))
    println(D(1).toString(2))
    println(D(1).hashCode("abc"))
    println((D(1): Any).hashCode == (D(2 - 1): Any).hashCode)
    println(D(1).equals(3, 3))
    println(D(1) == D(1))
    println(new P)
    println(new P().toString("a "))
    println(s"${D(5)} ${new P}")
