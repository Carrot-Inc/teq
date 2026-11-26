object Main { def main(args: Array[String]): Unit = {
var x = 10; val y = 3
class C { def m(a: Int = x + y): Int = a }; val c = new C
println(c.m()); x = 20; println(c.m())
}}
