object Main {
  def counter(): () => Int = {
    val next = {
      var n = 4
      () => { n += 1; n }
    }
    next
  }
  def main(args: Array[String]): Unit = {
    val next = counter()
    println(next())
    println(next())
  }
}
