object Main:
  def fromFinalizer(stop: Boolean): Int =
    try println("body")
    finally
      println("finally")
      if stop then return 21
    22

  def fromLock(stop: Boolean): Int =
    (if stop then return 31 else this).synchronized {
      println("locked")
      32
    }

  def main(args: Array[String]): Unit =
    println(fromFinalizer(true))
    println(fromFinalizer(false))
    println(fromLock(true))
    println(fromLock(false))
