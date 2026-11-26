object Main { def main(args: Array[String]): Unit = {
val x = 10; trait T { def m(a: Int = { def f(y: Int = x): Int = y; f() }): Int = a }; class C extends T; println(new C().m())
}}
