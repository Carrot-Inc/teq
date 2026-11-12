//> using platform jvm
// split-watch: jvm
// The names of local lazy vals' initialisers in a class's constructor trees, by dotty's per-unit counter in MegaPhase's
// order: a superclass's named arguments' temporaries in their written order, then a trait's, then the body (a
// member, a statement, an auxiliary constructor's self call and body, a method). Read by reflection, on the JVM.
class Base(val a: Int, val b: Int)
trait T(val t: Int, val u: Int)

class C(p: Int) extends Base(b = { lazy val x = p + 2; x }, a = { lazy val x = p + 1; x })
    with T(u = { lazy val x = 7; x }, t = { lazy val x = 8; x }):
  lazy val x = 10
  println({ lazy val x = 20; x })
  def this() =
    this({ lazy val x = 30; x })
    lazy val x = 40
    println(x)
  def m(): Int = { lazy val x = 50; x }

object Main:
  def main(args: Array[String]): Unit =
    val c = new C()
    println(c.a + c.b + c.x + c.m() + c.t + c.u)
    println(classOf[C].getDeclaredMethods.map(_.getName).filter(_.contains("lzyINIT")).sorted.mkString(","))
    println(classOf[C].getDeclaredFields.map(_.getName).sorted.mkString(","))
