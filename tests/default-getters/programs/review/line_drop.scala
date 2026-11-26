object Main { def main(args: Array[String]): Unit = {
val x = 10; trait A { def m(a: Int = x): Int = a }; trait B extends A { override def m(a: Int = x + 1): Int = a }; class C extends B { override def m(a: Int): Int = a * 2 }; println(new C().m())
}}
