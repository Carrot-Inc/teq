// A directory with a Java source: scala-cli compiles it (scalac
// reads it for its types, javac compiles it) and the program runs.
object K { def main(args: Array[String]): Unit = println("k " + J.v()) }
