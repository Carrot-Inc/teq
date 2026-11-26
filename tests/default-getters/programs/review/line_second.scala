object Main { def main(args: Array[String]): Unit = {
val x = 10; trait A { def m(a: Int = x, b: Int = x + 1): Int = a * 100 + b }; trait B extends A { override def m(a: Int, b: Int = x + 2): Int = a * 100 + b }; class C extends A with B; println(new C().m()); println(new C().m(b = 9))
}}
