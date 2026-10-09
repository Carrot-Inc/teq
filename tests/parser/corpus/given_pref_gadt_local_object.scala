class TC[+A](val n: Int)
enum K[A] {
  case I extends K[Int]
  case A extends K[Any]
}

def f[A](k: K[A]): Int = {
  object O {
    given a(using ValueOf[0]): TC[A] = new TC(1)
    given i: TC[Int] = new TC(2)
  }
  import O.given
  k match {
    case K.A => { val warm = summon[TC[Any]]; 0 }
    case K.I => summon[TC[Any]].n
  }
}

@main def probe(): Unit = println(f(K.I))
