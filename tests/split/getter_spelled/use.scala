package use

class Sub extends lib.Base {
  override def m(x: Int): Int = x + 100
}

@main def main(): Unit =
  println(new lib.Holder().h[String]())
  println((new Sub: lib.Base).m())
  println(new Sub().m())
