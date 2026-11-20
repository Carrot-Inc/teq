// One name entered twice in one scope of a Java source (E161): a class's type parameter, a method's type parameter,
// a parameter, each at the later one.
// expect: J.java:1:18: error: T is already defined as type T
// expect: J.java:1:33: error: A is already defined as type A
// expect: J.java:1:54: error: a is already defined as parameter a
object Use { def c = classOf[J[?, ?]] }
