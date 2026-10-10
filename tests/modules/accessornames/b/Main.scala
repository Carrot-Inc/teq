package names

object Main:
  def main(args: Array[String]): Unit =
    val a = new Api()
    println(a.inline$owner$$member)
    println(a.inline$plain)
    println(a.bump() + a.bump())
