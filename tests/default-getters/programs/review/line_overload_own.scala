object Main { def main(args: Array[String]): Unit = {
val x = 10; trait A { def m(a: Int = x): Int = a + x; def m(s: String): Int = s.length }; class C extends A { override def m(a: Int = 7): Int = a * 2; override def m(s: String): Int = s.length + 1 }; println(new C().m()); println(new C().m("ab"))
}}
