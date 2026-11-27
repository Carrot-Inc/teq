// The local trait is read from the upstream's pickle; scalac prints 20.
package esc
object Main { def main(args: Array[String]): Unit = println(Lib.make(8).m(2)()) }
