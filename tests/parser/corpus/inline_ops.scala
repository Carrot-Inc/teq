// Type-level operations on literal types, and givens that expand at each use.
import scala.compiletime.ops.int.*
import scala.compiletime.ops.string.{+ => ++}
import scala.compiletime.ops.boolean.{&&, ||, !}
import scala.compiletime.ops.any.{==, ToString}
import scala.compiletime.constValue

trait Render[A]:
  def render(a: A): String

object Render:
  inline given Render[Int] = new Render[Int]:
    def render(a: Int): String = "#" + a
  inline given [A](using inner: Render[A]): Render[List[A]] = new Render[List[A]]:
    def render(xs: List[A]): String = xs.map(inner.render).mkString("[", " ", "]")

object Limits:
  type Width = 8
  val cells: Width * Width = 64
  val next: S[Width] = 9
  val sum: 1 + 2 = 3
  val diff: 5 - 2 = 3
  val below: 1 < 2 = true
  val both: true && false = false
  val either: true || false = true
  val negated: ![false] = true
  val same: 1 == 1 = true
  val name: "col" ++ "umn" = "column"
  val shown: ToString[42] = "42"
  inline def area = constValue[Width * 2]
  inline def label[N <: Int] = constValue[ToString[N]]

object Main:
  def main(args: Array[String]): Unit =
    import Limits.*
    println(cells + next + sum + diff)
    println(below.toString + both + either + negated + same)
    println(name + shown)
    println(area)
    println(label[7] + label[Width])
    println(summon[Render[Int]].render(3))
    println(summon[Render[List[Int]]].render(List(1, 2)))
