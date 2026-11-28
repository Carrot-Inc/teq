object Main:
  def fail(): Unit = throw new IllegalStateException

  def intResult(): Int =
    try fail()
    catch case _: IllegalStateException => return 7
    finally println("int cleanup")
    -1

  def longResult(): Long =
    try fail()
    catch case _: IllegalStateException => return 4294967297L
    finally println("long cleanup")
    -1L

  def refResult(): String =
    try fail()
    catch case _: IllegalStateException => return "caught"
    finally println("reference cleanup")
    "fell through"

  def main(args: Array[String]): Unit =
    println(intResult())
    println(longResult())
    println(refResult())
