class V(val n: Int) extends AnyVal

object Main:
  def make(n: Int): V = new V(n)

  def name(n: Int): String =
    (1 + make(n)
      .n) match
      case 2 => "two"
      case _ => "other"

  def main(args: Array[String]): Unit =
    println(name(1))
    println(name(4))
