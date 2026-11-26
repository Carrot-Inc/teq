object Main { def main(args: Array[String]): Unit = {
var x = 10; var y = 3
class C { def m(a: Int = x + y): Int = a }; val c = new C
println(c.m()); x = 20; y = 7; println(c.m())
}}
