object Main { def main(args: Array[String]): Unit = {
var x = 10; trait T { def m(a: Int)(b: Int = x + a): Int = b }; val c = new T {}; println(c.m(2)()); x = 20; println(c.m(3)())
}}
