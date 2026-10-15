// A lambda among a block's statements takes the rest of the block as its body, as one at the block's
// start does (dotty's `blockStatSeq`, `closureRest` reading the body in `Location.InBlock`), an `implicit`
// one's too (`implicitClosure`): `{ implicit u => implicit v => .. }` is a lambda whose body is a lambda.
class U(val n: Int)
object Main {
  def nested[A](f: U => U => A): A = f(new U(3))(new U(4))
  def twice(f: Int => Int): Int = f(f(1))
  def main(args: Array[String]): Unit = {
    println(nested { implicit u => implicit v => summon[U].n })
    println(nested { u => v => u.n * 10 + v.n })
    println(twice { val k = 2; x => val y = x * k; y + 1 })
    println(nested { implicit u =>
      val a = summon[U].n
      v => a + v.n
    })
  }
}
