// The local classes' defaults the efp* cases call from their downstreams, one method a case.
trait API { def m(a: Int)(b: Int = a + 7): Int }
class Top { def m(a: Int)(b: Int = a + 7): Int = a + b }
object Lib {
  def bind(): Int = {
    val a = 100
    class C { def m(a: Int)(b: Int = a + 8): Int = a + b }
    new C().m(2)()
  }
  def poly(): String = {
    class C { def m[A](a: A)(b: A = a): A = b }
    new C().m[String]("ok")()
  }
  def self(): Int = {
    class C(val v: Int) { def m(a: Int = this.v): Int = a }
    new C(13).m()
  }
  def outer(): Int = {
    class O(val v: Int) { def get(): Int = {
      class C { def m(a: Int = O.this.v): Int = a }
      new C().m()
    }}
    new O(17).get()
  }
  def block(): Int = {
    var n = 1
    class C { def m(a: Int = { n += 1; n }): Int = a }
    val c = new C
    c.m() + c.m() + n
  }
  def byname(): Int = {
    var n = 0
    class C { def m(a: => Int = { n += 1; n }): Int = a + a }
    new C().m() * 10 + n
  }
  def contextual(): Int = {
    class C { def m(a: Int)(using b: Int = a + 8): Int = a + b }
    new C().m(2)
  }
  def ctor(): Int = {
    class C(val x: Int = 2) { def m(a: Int = 10): Int = x + a }
    new C().m()
  }
  def copy(): Int = {
    case class C(x: Int = 2)
    val c = C(9)
    c.copy().x + C().x
  }
  def extension(): Int = {
    class C {
      extension (x: Int) def m(y: Int = 7): Int = x + y
      def call(): Int = 2.m()
    }
    new C().call()
  }
  def inherited(): Int = {
    class C extends Top
    new C().m(2)()
  }
  def own(): API = {
    trait T extends API { override def m(a: Int)(b: Int = a + 8): Int = a + b }
    class C extends T
    new C
  }
  def none(): API = {
    trait T extends API { override def m(a: Int)(b: Int = a + 8): Int = a + b }
    class C extends T { override def m(a: Int)(b: Int): Int = a + b }
    new C
  }
  def anon(): API = {
    trait T extends API { override def m(a: Int)(b: Int = a + 8): Int = a + b }
    new T { override def m(a: Int)(b: Int = a + 9): Int = a + b }
  }
  def obj(): Int = {
    object O { def m(a: Int = 19): Int = a }
    O.m()
  }
  def overloaded(): Int = {
    class C { def m(a: Int = 22): Int = a; def m(s: String): Int = 99 }
    new C().m()
  }
}
