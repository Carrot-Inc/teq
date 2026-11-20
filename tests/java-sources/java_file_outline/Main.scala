// Every top-level declaration of a Java source is its package's class by its own name, not by the file's.
package p {
  object Run { def go(): Int = new Hidden().n() + new Second().m() }
}
object Main { def main(args: Array[String]): Unit = println(p.Run.go()) }
