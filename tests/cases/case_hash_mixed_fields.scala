object Main {
  case class Entry(key: String, value: Any)
  def main(args: Array[String]): Unit = {
    println(Entry("key", 7).hashCode)
    println(Entry("key", null).hashCode)
  }
}
