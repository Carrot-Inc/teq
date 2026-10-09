class TC[A](val n: Int)
object TC { given fallback: TC[Int] = new TC(0) }
enum Kind[A] {
  case I extends Kind[Int]
  case S extends Kind[String]
}

def test[A](k: Kind[A])(using tc: TC[A]): Int = k match {
  case Kind.S => { val warm = summon[TC[Int]]; 0 }
  case Kind.I => summon[TC[Int]].n
}

@main def main(): Unit =
  println(test(Kind.I)(using new TC[Int](2)))
