// The twin of inline_template_queued_last: the same definitions with the second inline method before the two
// bodies. Their results are declared, so each body is typed at its own place, after that method's check.
inline def first(a: Array[Int]): Int = a.length + 1
inline def last(a: Array[Int]): Int = a.length
def shown(): Unit = println(first(Array(1, 2)))
def again(): Unit = println(first(Array(3)))
object Main {
  def main(args: Array[String]): Unit = {
    shown()
    again()
  }
}
