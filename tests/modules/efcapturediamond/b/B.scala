// The local traits are read from the upstream's pickle; scalac prints 13.
package esc
object Main { def main(args: Array[String]): Unit = println(Lib.make(8).m(2)()) }
