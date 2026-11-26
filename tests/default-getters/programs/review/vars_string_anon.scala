object Main { def main(args: Array[String]): Unit = {
var x = "one"
trait T { def m(a: String = x): Int = a.length }; val c = new T {}
println(c.m()); x = "three"; println(c.m())
}}
