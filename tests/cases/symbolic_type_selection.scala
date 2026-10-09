// An operator-named type member selected through its owner, `K0.|:[F, G]` in shapeless3.
object K:
  type |:[A, B] = Either[A, B]
  type ++[A, B] = (A, B)
  object Nested:
    type ~>[F[_], G[_]] = [x] =>> F[x] => G[x]
object Main:
  def main(args: Array[String]): Unit =
    val e: K.|:[Int, String] = Right("r")
    val p: K.++[Int, String] = (1, "a")
    val f: K.Nested.~>[Option, List][Int] = (o: Option[Int]) => o.toList
    println(e.toString + " " + p + " " + f(Some(3)))
