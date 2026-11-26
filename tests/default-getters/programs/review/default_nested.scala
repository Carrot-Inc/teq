object Main { def main(args: Array[String]): Unit = {
val x = 10; trait T { trait U { def m(a: Int = x): Int = a }; def run(): Int = { class C extends U; new C().m() } }; class D extends T; println(new D().run())
}}
