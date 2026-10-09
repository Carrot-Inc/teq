// jars: cats-kernel-sjs cats-core-sjs
// `mapN` over a tuple of `Validated` whose error types differ (`ValidatedNec[Nothing, A]` from
// `validNec` next to a `ValidatedNec[E, B]`): the `F[_]` of the syntax joins the constructors
// as scalac does, and cats' `Hash` for the std's collections runs through MurmurHash3.
//> using dep org.typelevel::cats-core:2.13.0
import cats.*
import cats.data.*
import cats.syntax.all.*
enum Err:
  case Empty, Bad
final case class Form(name: String, age: Int, email: Option[String])
object Main:
  def name(s: String): ValidatedNec[Err, String] = if s.isEmpty then Err.Empty.invalidNec else s.validNec
  def age(n: Int): ValidatedNec[Err, Int] = if n < 0 then Err.Bad.invalidNec else n.validNec
  def main(args: Array[String]): Unit =
    val ok: ValidatedNec[Err, Form] = (name("a"), age(3), Option("e").validNec).mapN(Form.apply)
    val v2: ValidatedNec[Err, Form] = ("x".validNec, age(3), None.validNec).mapN(Form.apply)
    val v3: ValidatedNec[Err, Form] = (name(""), 3.validNec, Option.empty[String].validNec).mapN(Form.apply)
    val v4: ValidatedNec[String, (Int, String)] = (1.validNec, "s".validNec).mapN((a, b) => (a, b))
    val v5 = (name("a"), age(-1)).mapN((n, a) => s"$n $a")
    println(List(ok, v2, v3, v4, v5).mkString("\n"))
    println(Hash[List[Int]].hash(List(1, 2, 3)) == List(1, 2, 3).hashCode)
    println(Hash[Vector[Int]].hash(Vector(1, 2)) == Vector(1, 2).hashCode)
    println(Hash[Set[Int]].hash(Set(1, 2)) == Set(2, 1).hashCode)
    println(Hash[Option[String]].hash(Some("a")) == Some("a").hashCode)
