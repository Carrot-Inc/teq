// The last definition of the file is an inline method whose body calls a member the platform gives an array:
// its check is the last thing a worker types of the file, with nothing after it.
inline def first(a: Array[Int]): Int = a.length + 1
def shown(): Unit = println(first(Array(1, 2)))
def again(): Unit = println(first(Array(3)))
object Main {
  def main(args: Array[String]): Unit = {
    shown()
    again()
  }
}
inline def last(a: Array[Int]): Int = a.length
