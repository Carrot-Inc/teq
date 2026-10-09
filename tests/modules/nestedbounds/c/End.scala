package nbd
object End:
  def use(f: AnyRef { def outer(c: Ctx)(x: c.T): Parent[c.T] { def run[A <: c.T](a: A): A } }) = Mid.forward(f)
  def main(args: Array[String]): Unit =
    val f: AnyRef { def outer(c: Ctx)(x: c.T): Parent[c.T] { def run[A <: c.T](a: A): A } } = new AnyRef {
      def outer(c: Ctx)(x: c.T): Parent[c.T] { def run[A <: c.T](a: A): A } = new Parent[c.T] { def run[A <: c.T](a: A): A = a }
    }
    println(if use(f).eq(f) then "nested" else "other")
