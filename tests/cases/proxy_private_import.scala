// A binding the use cannot access is none a constructor proxy shadows (dotty's `findRef` keeps the
// accessible denotations alone): a private `def C` a wildcard import brings leaves `C(1)` the
// inner class's creation.
object Source:
  private def C(x: String): String = x
object Use:
  import Source.*
  class Inner:
    class C(val x: Int)
    val c = C(1)

@main def run(): Unit = println((new Use.Inner).c.x)
