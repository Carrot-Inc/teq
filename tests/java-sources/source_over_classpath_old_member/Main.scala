// The class path's J has old(); the program's Java source stands for J and has none: refused, as scalac refuses it.
object Main { def main(args: Array[String]): Unit = println(J.old()) }
