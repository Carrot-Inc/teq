object Layout:
  def f(flag: Boolean): Int =
    val value: Int =
      if flag then 4
      else throw new IllegalArgumentException("no")
    value + 1

object Main:
  def main(args: Array[String]): Unit = println(Layout.f(true))
