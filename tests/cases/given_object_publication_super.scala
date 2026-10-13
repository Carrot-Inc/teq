//> using platform jvm
// A given object's superclass constructor runs before the module is published: a read of the
// given there is `null` on the JVM (`MODULE$` is stored after the super call, `BCodeSkelBuilder`),
// and the interpreter answers it as the JVM does; Scala.js recurses there
// (tests/cases/given_object_publication_super_js). After the super call the given is the instance.
class Parent:
  val early: Parent = helper
def helper: Parent = t
given t: Parent with
  val late: Parent = helper
@main def run(): Unit =
  println(t.early == null)
  println(t.late eq t)
