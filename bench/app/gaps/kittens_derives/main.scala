//> using dep org.typelevel::kittens::3.5.0
import cats.*
import cats.derived.*
import cats.syntax.all.*
case class Slot(index: Int, label: String) derives Eq, Show
enum Tone derives Eq, Show:
  case Low, High
object Main:
  def main(args: Array[String]): Unit =
    println(Slot(1, "a") === Slot(1, "a"))
    println(Slot(1, "a").show)
    println((Tone.Low: Tone).show)
