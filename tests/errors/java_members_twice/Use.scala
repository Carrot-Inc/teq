// Two fields of one name, two methods of one erased signature in a Java source (E120), each at the later one.
// expect: J.java:1:43: error: Conflicting definitions:
// expect: J.java:1:86: error: Conflicting definitions:
object Use { def c = classOf[J] }
