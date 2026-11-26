object Main { def main(args: Array[String]): Unit = {
var n = 10; lazy val x = { println("lazy"); n + 1 }
trait T { def m(a: Int = x): Int = a }; class C extends T; val c = new C
println(c.m()); n = 20; println(c.m())
}}
