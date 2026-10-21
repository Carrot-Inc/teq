/*
rules = ["file:../../support/DumpFields.scala"]
*/
// The tree's field contract (docs/FIX.md, "Tree"): every kind's named fields in their order, empty ones, missing ones.
package fields

import scala.collection.mutable.{ListBuffer, Map as M, Set as _}
import scala.math.Ordering.Implicits.given

object FieldContract:
  val marker = 0 // assert: DumpFields.fields
  var v: Int = _
  final val (a, b) = (1, 2)
  def f[A](x: A, y: Int = 1)(using z: Int)(implicit w: String): A = x
  def g(xs: Int*): Int = -xs.length
  trait T { val t: Int; var u: Int; def d(x: Int): Int; type E }
  abstract class C[+A, -B](val c: Int) extends T with Serializable { self: T =>
    type F = Int
    def d(x: Int) = x
  }
  enum Color:
    case Red, Green
    case Mix(r: Int)
  given Ordering[Color] = Ordering.by(_.ordinal)
  given cmp: Ordering[String] with
    def compare(p: String, q: String) = 0
  export Color.Red
  val s = s"x$a y${b + 1}"
  val p = (a, b) match
    case (1, _) | (2, _) => 1
    case (x @ 3, y: Int) if y > 0 => x
    case _ => 0
  val l = for i <- List(1) if i > 0; j = i; (k, m) <- List((i, j)) yield k + m
  def loop(): Unit = for i <- List(1) do println(i)
  val fn = (x: Int) => x + 1
  val anon = List(1).map(_ + 1)
  val poly = [X] => (x: X) => x
  val typed: [X] => X => X = poly
  type L = [X] =>> List[X]
  val ann: Int @deprecated = 1
  val tr = try 1 catch { case e: Exception => 2 } finally ()
  val w = while v < 0 do v += 1
  val i = if a > 0 then 1 else 2
  val nw = new ListBuffer[Int]()
  val na = new T { val t = 1; var u = 2; def d(x: Int) = x; type E = Int }
  val asc = (1: Int)
  val eta = g _
  def ret(): Int = return 1
  def thr(): Nothing = throw new Exception("x")
  val unary = !true
  val sel = List(1).head
  val inf = 1 :: Nil
  val ty = List.empty[Int]
  val lits = (1L, 1.0, 1.0f, 'c', true, "s", (), null)
end FieldContract
