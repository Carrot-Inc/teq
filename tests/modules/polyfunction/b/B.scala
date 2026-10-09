package pfb

import pfa.A

@main def run(): Unit =
  println(A.use(A.pick))
  println(A.pick[Int](0)(List(7)))
  val run: A.Pick = [T] => (i: Int) => (xs: List[T]) => xs.reverse.drop(i).headOption
  println(A.use(run))
