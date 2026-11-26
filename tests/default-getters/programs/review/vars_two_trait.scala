object Main { def main(args: Array[String]): Unit = {
var x = 10; var y = 3
trait T { def m(a: Int = x + y): Int = a }; class C extends T; val c = new C
println(c.m()); x = 20; y = 7; println(c.m())
}}
