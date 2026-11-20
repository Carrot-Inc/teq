// A Java declaration naming a class of the program's Scala sources is refused at the name, in a member's signature;
// a `throws` clause names nothing the reader keeps, as the reference drops it.
// expect: J.java:2:10: error: not supported yet: a Java declaration naming S, a class of this program's Scala sources
// expect: J.java:2:14: error: not supported yet: a Java declaration naming S, a class of this program's Scala sources
class S extends Exception
object Use { def f(j: J, s: S): S = j.f(s) }
