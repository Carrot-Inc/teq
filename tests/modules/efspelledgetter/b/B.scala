// The local class is read from the upstream's pickle; scalac prints 7 then 99.
package esc
object Main { def main(args: Array[String]): Unit = { val c = Lib.make(); println(c.m()); println(c.other()) } }
