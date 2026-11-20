// A compact source file (members outside any type) is a form past the Java reader: refused at its first member, where
// the reference drops the whole unit.
// expect: C.java:1:1: error: not supported yet: a compact source file (the Java reader reads classes and interfaces)
object Use { def x = 1 }
