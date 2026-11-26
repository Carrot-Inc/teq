object Main { def main(args: Array[String]): Unit = {
val x = 10; class Outer { trait T { def m(a: Int = x): Int = a }; class C extends T; def run(): Int = new C().m() }; println(new Outer().run())
}}
