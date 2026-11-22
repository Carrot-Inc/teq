// A simple type name in a Java class's body that an import also gives is the member type a parent has
// (java.util.Map.Entry, through AbstractMap), as javac binds it; in the class's own header, where its members are not
// in scope, it is the import's (Comparable[q.Entry], a class of the class path: old/).
object Main {
  def main(args: Array[String]): Unit = {
    val j = new J
    println(s"${j.compareTo(new q.Entry)} ${J.pick(new java.util.AbstractMap.SimpleEntry("k", "v"))}")
  }
}
