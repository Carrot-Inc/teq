def f(): Int = { print("l"); 1 }
def r(): List[Int] = { print("r"); Nil }
@main def run(): Unit =
  println(M.shape(f() :: r()))
  println(M.shape(1 :: Nil))
  val xs = List(2)
  println(M.shape(1 :: xs))
