package lib
trait Base { def m(a: Int = 10): Int = a }
trait Parent extends Base { override def m(a: Int = 20): Int = a }
object Lib { trait T extends Base { override def m(a: Int = 21): Int = a }; class C extends T; def make(): Base = new C; def value(): Int = make().m() }
