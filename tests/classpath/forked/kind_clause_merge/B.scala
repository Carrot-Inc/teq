import cats.effect.kernel.Resource
object B:
  type Task[+A] = Either[Throwable, A]
  def v: Int =
    def k[G[_]](x: Int): Option[Resource[G, Int]] = None
    def s[G[_]](bytes: Array[Byte], name: String): Option[fs2.Stream[G, Byte]] = None
    val r: Option[Resource[Task, Int]] = None
    val t: Option[fs2.Stream[Task, Int]] = None
    M.one + k[Option](1).size + s[Option](Array.empty, "b").size + r.size + t.size
@main def run(): Unit = println(A.run() + B.v)
