// The local trait is read from the upstream's pickle; scalac prints 12 then 3.
package esc
object Main { def main(args: Array[String]): Unit = { val api = Lib.make(8); println(api.m(2)()); println(api.m("abc")) } }
