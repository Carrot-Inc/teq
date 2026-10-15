package msa

import scala.deriving.Mirror

// The mirrors scalac's `Synthesizer` makes, each summoned upstream and read downstream: a case
// class's companion and a generic case class's (cast to the instantiated refinement), an enum's
// companion and its singleton case, a sealed trait's written companion, an anonymous mirror of a
// sealed trait without one, a case object, a tuple's `TupleMirror`; `derives` clauses over them.
trait Make[A]:
  def make(p: Product): A

object Make:
  def derived[A](using m: Mirror.ProductOf[A]): Make[A] = new Make[A]:
    def make(p: Product): A = m.fromProduct(p)

trait Index[A]:
  def index(a: A): Int

object Index:
  def derived[A](using m: Mirror.SumOf[A]): Index[A] = new Index[A]:
    def index(a: A): Int = m.ordinal(a)

case class Plain(x: Int, s: String) derives Make
case class Generic[A](x: A) derives Make

enum Sum derives Index:
  case One
  case Many(n: Int)

sealed trait Shape
case class Circle(r: Int) extends Shape
case object Dot extends Shape

sealed trait Tagged
object Tagged:
  def describe: String = "tagged"
case class Red(x: Int) extends Tagged
case class Blue() extends Tagged

case object Lone

object Use:
  val plain = summon[Mirror.ProductOf[Plain]]
  val generic = summon[Mirror.ProductOf[Generic[Int]]]
  val sum = summon[Mirror.SumOf[Sum]]
  val one = summon[Mirror.ProductOf[Sum.One.type]]
  val many = summon[Mirror.ProductOf[Sum.Many]]
  val shape = summon[Mirror.SumOf[Shape]]
  val tagged = summon[Mirror.SumOf[Tagged]]
  val lone = summon[Mirror.ProductOf[Lone.type]]
  val pair = summon[Mirror.ProductOf[(Int, String)]]
