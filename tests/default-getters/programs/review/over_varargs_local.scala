object Main { def main(args: Array[String]): Unit = {
val x = 10; trait T { def m(a: Int = x): Int = a + x; def m(xs: Int*): Int = xs.sum + 100 }; class C extends T; val c = new C; println(c.m()); println(c.m(4)); println(c.m(4, 5))
}}
