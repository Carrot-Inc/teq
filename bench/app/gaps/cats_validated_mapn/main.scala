//> using dep org.typelevel::cats-core::2.13.0
import cats.data.ValidatedNec
import cats.syntax.all.*
object Main:
  def main(args: Array[String]): Unit =
    val v: ValidatedNec[String, Int] = 3.validNec
    val w: ValidatedNec[String, String] = "bad".invalidNec
    println((v, w).mapN((i, s) => s"$i$s"))
    println(List(1, 2).traverse(i => Option(i)))
