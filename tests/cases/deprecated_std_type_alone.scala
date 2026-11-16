// A type is the program's one name of a deprecated definition of the std: the name's own target makes it
// checked, and the build's summary counts it.
object Main {
  def size(s: Stream[Int]): Int = if s == null then 0 else 1
  def main(a: Array[String]): Unit = println(size(null))
}
