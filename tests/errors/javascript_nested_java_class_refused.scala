// jars: fixtures fixtures-rdecl javafix
// command: build
// expect: not supported on JavaScript: class Nested fix.Outer.Nested (a Java definition without an implementation in the std's platform layer)
// A library's body (fix.reader.RdCalls.javaNested, from its jar's TASTy) that creates a subclass of a static member
// class of a jar's Java class (fix.Outer.Nested of tests/classfile's fixtures), built for JavaScript: the member
// class is read from its class file as the Java class it is, and the JavaScript target, which has no Java classes,
// refuses it by name where the build reaches it, where the JavaScript would name a class it does not define.
object Main {
  def main(args: Array[String]): Unit = println(fix.reader.RdCalls.javaNested)
}
