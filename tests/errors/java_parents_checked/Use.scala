// A Java source's parents as the reference checks them: a cycle through two sources (E110, once, at the first
// class's parent), a final class extended (E093, at the class), a class implemented (E104, at its name).
// expect: A.java:1:24: error: Cyclic inheritance: class A extends itself
// expect: F.java:1:14: error: class F cannot extend final class String
// expect: I.java:1:27: error: class Object is not a trait
object Use { def c = classOf[F] }
