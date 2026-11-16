// jars: scala-library javactors-lib
// std: scala-library
// targets: jvm
// A Java class's protected or package-private constructor is its package's, a protected one a subclass's
// super call's too (a parent call, an anonymous subclass with a body or with none), and of overloaded
// constructors a creation takes one the place may call (tests/support/java_ctors). An anonymous subclass
// of an empty body is a class of its own where the JVM guards the parent's constructor: a JDK class's
// protected one is called from it.
package jlib:
  class SameSub extends Local(4)
  object Same:
    def all: String =
      List(new Guarded(1).x, new Local(2).x, new SameSub().x, (new Guarded(5) {}).x).mkString(" ") + " " +
        new Many("a").s + " " + new Many(Integer.valueOf(1)).s

package other:
  class Sub extends jlib.Guarded(7)
  class SubS extends jlib.Many("a")
  class SubI extends jlib.Many(Integer.valueOf(1))
  class SubShelf extends jlib.Shelf[Int]
  object Out:
    def all: String =
      List(new Sub().x, (new jlib.Guarded(8) { def y = 1 }).x, (new jlib.Guarded(9) {}).x).mkString(" ") + " " +
        List(new jlib.Many("a").s, new jlib.Many(Integer.valueOf(1)).s, new SubS().s, new SubI().s).mkString(" ") + " " +
        (new jlib.Boxed[String]("g") {}).x + " " + new SubShelf().v + (new jlib.Shelf[Int] {}).v + " " + (new jlib.Open {}).v + " " +
        (new jlib.Spread() {}).n + (new jlib.Spread("a", "b") {}).n

@main def main(): Unit =
  println(jlib.Same.all)
  println(other.Out.all)
  println((new java.io.ObjectOutputStream() {}).getClass.getSuperclass.getName)
