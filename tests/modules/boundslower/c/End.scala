package bnl
object End:
  def use(c: Ctx)(f: Parent[c.T] { def run[A >: c.T](a: A): A }) = Mid.forward(c)(f)
  def poly(c: Ctx) = Mid.poly(c)
  def main(args: Array[String]): Unit =
    val c = new Ctx { type T = String }
    val p: Parent[c.T] { def run[A >: c.T](a: A): A } = new Parent[c.T] { def run[A >: c.T](a: A): A = a }
    println(use(c)(p).run[Any]("bounded"))
