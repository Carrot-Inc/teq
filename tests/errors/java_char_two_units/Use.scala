// A Java character literal is one UTF-16 unit: an escaped pair is two, and the literal is unclosed after the first.
// expect: J.java:1:47: error: unclosed character literal
object Use { def x = J.X }
