class V(val i: Int) extends AnyVal

object Main:
  def id[A](value: A): A = value

  def generic(v: V): Long =
    id[V](v)
      .i

  def cast(v: Any): Int =
    v.asInstanceOf[V]
      .i

  def main(args: Array[String]): Unit =
    println(generic(new V(3)))
    val boxed: Any = new V(4)
    println(cast(boxed))
