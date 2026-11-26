object Main { def main(args: Array[String]): Unit = {
var x = "one"
class C { def m(a: String = x): Int = a.length }; val c = new C
println(c.m()); x = "three"; println(c.m())
}}
