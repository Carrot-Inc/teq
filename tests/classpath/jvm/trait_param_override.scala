// A trait parameter whose accessor the class or a trait mixed in after overrides (dotty's `Mixin`, 301-303): the
// argument is evaluated and dropped, the trait's `$init$` reads the overriding val before the class sets it (0), and
// a class over one that implements the trait already initialises the trait no second time. A later trait's constant
// overrides it too, its getter the trait's own.
def side(s: String): Int = { println("arg " + s); s.length }
trait T(val x: Int):
  println("init " + x)
class C extends T(side("c")):
  override val x: Int = 9
class D extends C with T
trait Over(val o: Int):
  println("over " + o)
trait Later extends Over:
  override val o: Int = 77
class L extends Over(side("l")) with Later
class P(override val x: Int) extends T(side("p"))
trait Fixed extends T:
  override final val x = 9
class F extends T(side("f")) with Fixed
@main def main(): Unit =
  println(new D().x)
  println(new L().o)
  println(new P(5).x)
  val f = new F
  println((f.x, (f: T).x))
