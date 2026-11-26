object Main { def main(args: Array[String]): Unit = {
val x = 10; trait T { def m(a: Int = x): Int = a; def m(a: Int, b: Int = 3): Int = a + b }; class C extends T; println(new C().m(2))
}}
