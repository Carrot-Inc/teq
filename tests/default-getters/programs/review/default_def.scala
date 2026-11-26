object Main { def main(args: Array[String]): Unit = {
var x = 10; def get(): Int = { println("get"); x }; trait T { def m(a: Int = get()): Int = a }; class C extends T; val c = new C; println(c.m()); x = 20; println(c.m())
}}
