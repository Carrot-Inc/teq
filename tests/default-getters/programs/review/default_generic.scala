object Main { def main(args: Array[String]): Unit = {
val x = 10; trait T { def m[A](a: A)(b: A = a, c: Int = x): String = b.toString + c }; class C extends T; println(new C().m("a")())
}}
