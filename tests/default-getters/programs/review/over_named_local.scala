object Main { def main(args: Array[String]): Unit = {
val x = 10; trait T { def m(a: Int = x): Int = a + x; def m(s: String): Int = s.length }; class C extends T; val c = new C; println(c.m()); println(c.m(a = 4)); println(c.m(s = "abc")); val t: T = c; println(t.m()); println(t.m(a = 4))
}}
