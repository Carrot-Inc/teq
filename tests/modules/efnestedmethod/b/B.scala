// The class the downstream calls is read from the upstream's pickle; scalac prints 22.
package esc
object Main { def main(args: Array[String]): Unit = println(Lib.make(10).m(2)()) }
