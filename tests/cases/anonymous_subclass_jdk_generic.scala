// An anonymous subclass of an empty body of a generic JDK class whose constructor is protected: named with its
// type arguments and through a generic alias applied. Each is the class's super call; on the JVM a class of
// its own.
type V[T] = java.nio.file.SimpleFileVisitor[T]
def named(v: java.nio.file.SimpleFileVisitor[String]): String = if v != null then "made" else "none"
@main def main(): Unit =
  println(named(new java.nio.file.SimpleFileVisitor[String]() {}))
  println(named(new V[String]() {}))
  val keep: java.nio.file.FileVisitor[String] = new java.nio.file.SimpleFileVisitor[String]() {}
  println(keep != null)
