// A local trait's override's default reading nothing of the block, left through a local class's super call: scalac
// prints 22.
package esc
object Main { def main(args: Array[String]): Unit = println(Lib.make(10).m(2)()) }
