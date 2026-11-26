object Main { def main(args: Array[String]): Unit = {
val x = 10; trait T { def m(a: Int = { trait U { def n(b: Int = x): Int = b }; class D extends U; new D().n() }): Int = a }; class C extends T; println(new C().m())
}}
