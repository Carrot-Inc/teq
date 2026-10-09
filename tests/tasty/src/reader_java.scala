package fix.rjava

// A static nested class of a Java class (`fix.Outer.Nested` of tests/classfile), a member of
// its companion in scalac's signatures (`fix.Outer$.Nested`), as a parameter of one name's
// overloads; reader_java_v1.scala, which the bodies of reader.scala were compiled against, has
// the first alone.
class RdSub extends fix.Outer.Nested

object RdJava:
  def f(n: fix.Outer.Nested): String = "f(Outer.Nested)"
  def f(s: RdSub): String = "f(RdSub)"
