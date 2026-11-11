trait Counter:
  def read(): Int
inline def twice(counter: Counter): Int = counter.read() + counter.read()
def count(text: String): Int = twice {
  class Local(val amount: Int) extends Counter:
    def this(text: String) = this(text.length)
    def read(): Int = amount
  new Local(text)
}
object Main:
  def main(args: Array[String]): Unit =
    println(count("abc"))
    println(count("hello"))
