object Main { def main(args: Array[String]): Unit = {
class Box(val n: Int); var x = new Box(10)
trait T { def m(a: Box = x): Int = a.n }; val c = new T {}
println(c.m()); x = new Box(20); println(c.m())
}}
