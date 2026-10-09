trait Show[A]
given Show[Int] with {}
def f[A](using Show[A]): Unit = ()
@main def main(): Unit = f
