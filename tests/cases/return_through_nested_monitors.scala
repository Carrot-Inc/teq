object Main:
  val outer = new Object
  val inner = new Object

  def nested(n: Int): Int =
    outer.synchronized {
      inner.synchronized {
        println("inner " + n)
        if n > 0 then return n
      }
      println("after inner")
    }
    -1

  def main(args: Array[String]): Unit =
    println(nested(9))
    println(nested(0))
