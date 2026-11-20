// command: build
// A method of a Java source has its body from javac, which JavaScript does not have: a call reached is refused, naming
// the source.
// expect: J.java:1:1: error: not supported on JavaScript: object J of the Java source tests/errors/java_body_on_javascript/J.java, whose body JavaScript does not have
object Use { def main(args: Array[String]): Unit = println(J.n()) }
