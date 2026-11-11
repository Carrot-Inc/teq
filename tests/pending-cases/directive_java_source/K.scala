// A directory with a Java source: scala-cli compiles it (scalac reads it for its types, javac compiles it) and the
// program runs; teq reads Java class files only and takes no .java file of a directory.
object K { def main(args: Array[String]): Unit = println("k " + J.v()) }
