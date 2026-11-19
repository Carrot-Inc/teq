// expect: 9:29: error: No given instance of type TC[List] was found for parameter x of method summon in package scala
// expect: 1 error found
type Const[T] = [X] =>> T
trait TC[F[_]]
object TC:
  given constTC[T]: TC[Const[T]] = new TC[Const[T]] {}
object Main:
  def main(args: Array[String]): Unit =
    println(summon[TC[List]])
