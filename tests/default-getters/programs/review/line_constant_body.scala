object Main { def main(args: Array[String]): Unit = {
val x = 10; trait A { def m(a: Int = x): Int = a }; trait B extends A { override def m(a: Int = 7): Int = a + x }; class C extends A with B; println(new C().m())
}}
