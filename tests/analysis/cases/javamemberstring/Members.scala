package demo

// A file that reaches String only through a Java member: a super call to a Throwable's constructor taking a String, a
// Throwable's getMessage, an Exception's constructor (the subclasses local, their API none of the file's). scalac
// records no dependency on Predef for any of them.
object F { val text = "t"; def fail(): Throwable = { class Failure extends Throwable(text); new Failure } }
object G { def m(t: Throwable): Unit = { val s = t.getMessage; () } }
object H { def refused(n: Int): Exception = { class Refused extends Exception("refused"); new Refused } }
