// The same left through a local class's override calling super, its second clause's default reading the first: scalac
// prints 22. The readers' contract: tests/modules/efpure/a/A.scala.
package lp_escapeprior
object Caller { def call(t: API): Int = t.m(2)() }
object Main { def main(args: Array[String]): Unit = println(Caller.call(Lib.make(10))) }
