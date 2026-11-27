import a2.*
object Run { def main(args: Array[String]): Unit = { D(using new I { def show(x: String) = println(s"D: $x")}); E(using new I { def show(x: String) = println(s"E: $x")}); F(using new I { def show(x: String) = println(s"F: $x")}); class G(using I) extends F; G(using new I { def show(x: String) = println(s"G: $x")}); Api.g(using new I { def show(x: String) = println(s"g: $x")}) } }
