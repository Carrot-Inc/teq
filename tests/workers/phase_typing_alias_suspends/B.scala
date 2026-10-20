object B:
  type T = A.inferred.type
  val x: T = A.inferred
@main def run(): Unit = println(B.x + 1)
