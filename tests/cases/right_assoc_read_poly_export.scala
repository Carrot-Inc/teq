// A right-associative extension with a type parameter and a using clause before its receiver, written out with its
// type arguments interleaved (`[A](using u)[B](r)(l)`), directly and through an export under another name.
object E:
  extension [A](using u: Boolean)(l: A)
    def +:[B](r: B): String = { println("body"); r.toString + l.toString }
object F:
  export E.{+: as join}
def n(): Int = { println("left"); 1 }
def s(): String = { println("right"); "r" }
def b(): Boolean = { println("using"); true }
@main def run(): Unit =
  println(E.+:[Int](using b())[String](s())(n()))
  println(F.join[Int](using b())[String](s())(n()))
