// A simple type name in a Java class's body that an import resolves to the very member type a parent of the class has
// (java.util.Map.Entry, imported, in a class extending AbstractMap): the class reads as javac reads it.
object Main {
  def main(args: Array[String]): Unit = {
    val j = new J
    println(J.key(java.util.Map.entry("a", "b")))
    println(j.entrySet().size)
    println(j.get("k"))
  }
}
