object Main { def main(args: Array[String]): Unit = {
var x = 10; class C { def m(a: Int)(b: Int = x + a): Int = b }; val c = new C; println(c.m(2)()); x = 20; println(c.m(3)())
}}
