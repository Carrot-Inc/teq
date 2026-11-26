object Main { def main(args: Array[String]): Unit = {
var x = 10; trait T { def m(a: => Int = { x += 1; x }): Int = a + a }; class C extends T; println(new C().m()); println(x)
}}
