object A:
  type F[X] = List[X]
  type G[X] = Map[X, X]
@main def appliedAlias(): Unit =
  println(Reflect.shape[scala.ImpureFunction1[Int, Int]])
  println(Reflect.shape[A.F[Int]])
  println(Reflect.shape[A.G[Int]])
