// A source with a member outside any type (a compact source file) is dropped whole, as scalac drops it: its class is
// not found.
object Main {
  def main(args: Array[String]): Unit = println(new Helper().v())
}
