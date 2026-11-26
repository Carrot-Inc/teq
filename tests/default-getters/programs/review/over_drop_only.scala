object Main { def main(args: Array[String]): Unit = {
val x = 10; trait T { def m(a: Int = x): Int = a + x; def m(s: String): Int = s.length }; class C extends T; class D extends C { override def m(a: Int): Int = a * 3 }; println(new D().m())
}}
