object Main { def main(args: Array[String]): Unit = {
val x = 10; trait A { def m(a: Int = x): Int = a }; trait B extends A { override def m(a: Int = x + 1): Int = a }; trait D extends A { override def m(a: Int = x + 2): Int = a }; class C extends B with D; println(new C().m())
}}
