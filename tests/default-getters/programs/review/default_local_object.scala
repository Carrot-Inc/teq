object Main { def main(args: Array[String]): Unit = {
var x = 10; trait T { def m(a: Int = x): Int = a }; object C extends T; println(C.m()); x = 20; println(C.m())
}}
