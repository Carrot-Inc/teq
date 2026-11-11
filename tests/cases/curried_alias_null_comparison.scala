type Const[A] = [X] =>> A
def isNull(x: Const[Any][Int]): Boolean = x == null
@main def run(): Unit = println(isNull(null))
