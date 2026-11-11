import holders.{*, given}

object Main:
  def main(args: Array[String]): Unit =
    println(increment(summon[Int]))
    println(decorate(summon[String]))
    println(summon[Boolean])
