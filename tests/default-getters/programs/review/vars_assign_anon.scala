object Main { def main(args: Array[String]): Unit = {
var x = 10
trait T { def m(a: Int = { x += 1; x }): Int = a }; val c = new T {}
println(c.m()); x = 20; println(c.m())
}}
