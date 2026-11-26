object Main { def main(args: Array[String]): Unit = {
val x = 10; trait T { def m(a: Int = x): Int = a + x; def m(s: String = "a"): Int = s.length }; class C extends T; println(new C().m())
}}
