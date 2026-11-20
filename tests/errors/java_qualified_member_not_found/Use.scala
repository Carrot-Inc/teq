// A qualified name whose segment names nothing binds no class: each use is refused at that segment in the
// reference's words, and no check reads through it (no "class Outer is not a trait" for the parent, no conflict
// between the two `g`).
// expect: J.java:3:33: error: type Absent is not a member of object p.Outer
// expect: J.java:4:16: error: type Missing is not a member of object p.Outer
// expect: J.java:5:23: error: type Missing is not a member of object p.Outer
// expect: J.java:6:23: error: type Other is not a member of object p.Outer
// expect: J.java:7:24: error: type Nope is not a member of object java.util.Map
// expect: J.java:8:15: error: value nope is not a member of java
// expect: J.java:9:10: error: Not found: nope
object Use { def f = new p.J }
