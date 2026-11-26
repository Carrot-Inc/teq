object Main { def main(args: Array[String]): Unit = {
val x = 10; trait T { def m(a: Int = x): Int = a + x }; class C extends T; println(new C().m("bad"))
}}
