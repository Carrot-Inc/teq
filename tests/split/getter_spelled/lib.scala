package lib

class Base {
  def m(x: Int = 7): Int = x
}

class Holder {
  def h[A](x: A = "seven".asInstanceOf[A]): A = x
}
