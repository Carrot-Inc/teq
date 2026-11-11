// A package's extension method is a member of its package object, which a selection on the package's path finds and
// applies to its receiver as any method (dotty's PackageClassDenotation.computeMembersNamed).
package p:
  extension (x: Int) def pk: Int = x + 1
  extension (x: Int) def pk2(y: Int): Int = x + y
  package q:
    extension (s: String) def shout: String = s.toUpperCase + "!"

@main def run(): Unit =
  println(p.pk(3))
  println(p.pk2(3)(4))
  println(p.q.shout("hi"))
  println(_root_.p.pk(1))
