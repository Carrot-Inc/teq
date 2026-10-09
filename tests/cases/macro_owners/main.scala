package demo.owners

import Owners.chain

def show(tag: String)(using s: Site): String = tag + ":" + s.chain

val topVal = chain
val topTyped: String = chain
lazy val topLazy = chain
def topDef: String = chain
val topLambda: Int => String = x => chain

trait Show[A]:
  def show(a: A): String

given Show[Int] with
  def show(a: Int): String = chain

given [A] => Show[List[A]] = new Show[List[A]]:
  def show(a: List[A]): String = "list"

given topGiven: String = chain

trait TextKey[T]
trait LongKey[T]
given [T: TextKey] => Ordering[T] =
  println(show("anon1"))
  Ordering.by(_.toString)
given [T: LongKey] => Ordering[T] =
  println(show("anon2"))
  Ordering.by(_.hashCode)

class Widget(val id: Int):
  val field = chain
  println(show("classBody"))
  println("lambdaInBody:" + List(1).map(_ => chain).head)
  def method: String = chain
  def withLocals: String =
    def local(y: Int): String =
      def deeper: String = chain
      chain + "\n" + deeper
    val localVal = chain
    val lambda = (z: Int) => chain
    val (first, second) = (chain, 2)
    val (c, d): (String, Int) = (chain, 2)
    val Some(e) = Some(chain): @unchecked
    val inBlock = { val inner = chain; inner }
    List(local(1), localVal, lambda(1), first, c, e, inBlock).mkString("\n")
  def inFor: List[String] =
    for
      x <- List(1)
      y = chain
    yield y
  def withLocalClass: String =
    class Local:
      val inLocal = chain
    Local().inLocal

object Outer:
  val inObject = chain
  println(show("objectBody"))
  object Inner:
    def deep: String = chain
  given Show[Boolean] with
    def show(a: Boolean): String = chain

extension (x: Int)
  def ext: String = chain

def localGiven: String =
  given String = chain
  summon[String]

var topVar = chain

case class Key(k: Int)
object Key:
  given TextKey[Key] = new TextKey[Key] {}
case class LKey(k: Int)
object LKey:
  given LongKey[LKey] = new LongKey[LKey] {}

@main def run(): Unit =
  println("topVal:" + topVal)
  println("topTyped:" + topTyped)
  println("topLazy:" + topLazy)
  println("topDef:" + topDef)
  println("topLambda:" + topLambda(1))
  println("given:" + summon[Show[Int]].show(1))
  println("topGiven:" + topGiven)
  summon[Ordering[Key]]
  summon[Ordering[LKey]]
  val w = Widget(1)
  println("field:" + w.field)
  println("method:" + w.method)
  println("withLocals:" + w.withLocals)
  println("inFor:" + w.inFor.head)
  println("localClass:" + w.withLocalClass)
  println("inObject:" + Outer.inObject)
  println("deep:" + Outer.Inner.deep)
  println("inObjectGiven:" + Outer.given_Show_Boolean.show(true))
  println("ext:" + 1.ext)
  println("localGiven:" + localGiven)
  println("topVar:" + topVar)
  println(show("inMain"))
