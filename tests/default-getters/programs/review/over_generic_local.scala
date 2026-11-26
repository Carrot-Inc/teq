object Main { def main(args: Array[String]): Unit = {
val x = 10; trait T { def m(a: Int = x): Int = a + x; def m[A](a: A): Int = 99 }; class C extends T; val c = new C; println(c.m()); println(c.m(4)); println(c.m("s"))
}}
