import lib.*
object Main { def main(args: Array[String]): Unit = { class C extends Parent { override def m(a: Int = 30): Int = a + 100 }; class D extends Parent { override def m(a: Int): Int = a + 100 }; println((new C(): Base).m()); println((new D(): Base).m()); println(Lib.make().m()); println(Lib.value()) } }
