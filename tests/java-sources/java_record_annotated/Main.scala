// A top-level record whose modifiers begin with an annotation is read as javac reads it, where scalac reading the
// source refuses it ("illegal start of type declaration"): a Java source's validity is javac's (docs/COMPATIBILITY.md).
object Main {
  def main(args: Array[String]): Unit = println(new R(7).x() + new Q(1).y())
}
