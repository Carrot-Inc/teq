object O {
  implicit def f[A]: Int = 1
  implicit def g: String = ""
  implicit def k[A](using a: A): Int = 1
  implicit def m(using s: String): Int = 1
}
