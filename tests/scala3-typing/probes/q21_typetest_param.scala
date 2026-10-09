object T:
  def f[T](x: Any): Boolean = x match
    case _: T => true
    case _ => false
  def g[T](x: Any): Boolean = x.isInstanceOf[T]
  def h(x: Any): Boolean = x match
    case _: List[String] => true
    case _ => false
@main def run(): Unit = println(T.f[Int]("s"))
