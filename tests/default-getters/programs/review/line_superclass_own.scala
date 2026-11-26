object Main { def main(args: Array[String]): Unit = {
val x = 10; trait A { def m(a: Int = x): Int = a }; class C extends A { override def m(a: Int = 7): Int = a * 2 }; class D extends C; println(new D().m())
}}
