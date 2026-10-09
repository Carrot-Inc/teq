// `xs reverse_::: ys` prepends the reversed `xs` to `ys`.
@main def run(): Unit =
  println(List(1, 2, 3) reverse_::: List(4, 5))
  println(Nil reverse_::: List("a"))
  println(List('x').reverse_:::(List('y', 'z')))
