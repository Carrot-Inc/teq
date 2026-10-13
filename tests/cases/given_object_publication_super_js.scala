//> using platform js
// A given object over a class with a body (dotty's module): Scala.js publishes the module after the
// super call, which runs the superclass's body (`JSCodeGen`, 2308-2313), so a read of the given
// from that body makes it anew and recurses (tests/cases/given_object_publication_super, the JVM's).
class Parent:
  val early: Parent = helper
def helper: Parent = t
given t: Parent with
  val late: Parent = helper
@main def run(): Unit =
  val r = try { if t.early == null then "null" else "instance" } catch case e: Throwable => "overflow"
  println(r)
