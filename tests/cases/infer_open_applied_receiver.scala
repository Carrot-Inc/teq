// A member selected on an application's result (`Response(status).withEntity(b)(using enc)` for
// a `Response[F[_]]`): a type argument nothing constrains and the result holds invariantly
// stays open for the selection's using clause, as scalac leaves it, rather than `Nothing`.
class Enc[F[_]]
class R[F[_]]:
  def withE(using e: Enc[F]): R[F] = this
  def plain: R[F] = this
object R:
  def apply[F[_]](i: Int): R[F] = new R[F]
given Enc[Option] = Enc()
object Main:
  def main(args: Array[String]): Unit =
    val r: R[Option] = R(1).plain.withE(using summon[Enc[Option]])
    println(r != null)
