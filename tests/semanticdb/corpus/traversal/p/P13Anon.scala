package p13
trait A { def f: Int }
object O:
  val a = new A { def f = 1; val g = 2 }
  def m = new A:
    def f = 2
  object Inner
  case object CO
  val x = new Object
  def loc: Int =
    val an = new A { def f = 3 }
    object LO { val q = 1 }
    an.f + LO.q
class WithCompanion
object WithCompanion
