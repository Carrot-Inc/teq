import scala.collection.mutable

trait Enumerated[E]:
  def values: List[E]
  extension (e: E) def entryName: String

object Models:
  enum CollectionWay(val entryName: String):
    case Doorstep extends CollectionWay("doorstep")
    case ViaPost extends CollectionWay("via-post")
    case FrontDesk extends CollectionWay("front_desk")

  object CollectionWay:
    val default: CollectionWay = ViaPost
    def byName(n: String): Option[CollectionWay] = values.find(_.entryName == n)
    given Enumerated[CollectionWay] with
      def values: List[CollectionWay] = CollectionWay.values.toList
      extension (e: CollectionWay) def entryName: String = "mirror:" + e.toString

  enum RemoteRecord[+A]:
    case NotLoaded, Loading
    case Available(value: A)
    case Failed(message: String, retries: Int = 0)

    def map[B](f: A => B): RemoteRecord[B] = this match
      case Available(v) => Available(f(v))
      case NotLoaded => NotLoaded
      case Loading => Loading
      case Failed(m, r) => Failed(m, r)

    def toOption: Option[A] = this match
      case Available(v) => Some(v)
      case _ => None

    def isBusy: Boolean = this == Loading

import Models.*
import Models.RemoteRecord.*

enum Op(val symbol: Char, val run: (Int, Int) => Int):
  case Add extends Op('+', _ + _)
  case Mul extends Op('*', _ * _)
  case Sub extends Op('-', (a, b) => a - b)
  override def toString: String = s"Op($symbol)"

enum Json:
  case JNull
  case JBool(b: Boolean)
  case JNum(n: Int)
  case JArr(items: List[Json])
  case JObj(fields: List[(String, Json)])

  def render: String = this match
    case JNull => "null"
    case JBool(b) => b.toString
    case JNum(n) => n.toString
    case JArr(items) => items.map(_.render).mkString("[", ",", "]")
    case JObj(fields) => fields.map((k, v) => "\"" + k + "\":" + v.render).mkString("{", ",", "}")

def names[E](es: List[E])(using m: Enumerated[E]): List[String] = es.map(_.entryName)

def status(r: RemoteRecord[Int]): String = r match
  case NotLoaded | Loading => "pending"
  case Available(v) if v > 10 => "big " + v
  case Available(v) => "small " + v
  case Failed(m, 0) => "failed: " + m
  case Failed(m, n) => s"failed $n times: $m"

@main def main(): Unit =
  println(CollectionWay.values.toList.map(_.entryName))
  println(names[CollectionWay](CollectionWay.values.toList))
  println(CollectionWay.byName("via-post"))
  println(CollectionWay.byName("nope"))
  println(CollectionWay.default.ordinal)
  println(CollectionWay.valueOf("FrontDesk").entryName)
  println(CollectionWay.fromOrdinal(0))
  println(List(Available(3), Loading, Failed("x"), Failed("y", 2), NotLoaded, Available(30)).map(status))
  println(Available(2).map(_ * 25))
  println((Loading: RemoteRecord[Int]).map(_ + 1).isBusy)
  println(Available("a").toOption.toList ++ NotLoaded.toOption.toList)
  println(Available(1) == Available(1))
  println(Available(1).hashCode == Available(1).hashCode)
  println(Failed("m") == Failed("m", 0))
  println(Failed("m").copy(retries = 3))
  println(Failed("m").productPrefix + " " + Loading.productPrefix + " " + Loading.ordinal + " " + Failed("m").ordinal)
  println(Op.values.toList.map(op => s"$op=${op.run(6, 3)}"))
  println(Op.values.map(_.symbol).mkString)
  val tally = mutable.Map[RemoteRecord[Int], Int]()
  for r <- List(Loading, Available(1), Loading, Available(1), Available(2), NotLoaded) do
    tally(r) = tally.getOrElse(r, 0) + 1
  println(tally.toList.map((k, v) => s"$k:$v").sorted)
  val set: Set[CollectionWay] = Set(CollectionWay.Doorstep, CollectionWay.ViaPost, CollectionWay.Doorstep)
  println(set.size)
  println(set.contains(CollectionWay.FrontDesk))
  val byOption: Map[CollectionWay, Int] = CollectionWay.values.toList.map(o => o -> o.entryName.length).toMap
  println(byOption(CollectionWay.FrontDesk))
  println(CollectionWay.values.toList.sortBy(_.entryName).map(_.toString))
  println(Json.JObj(List("a" -> Json.JArr(List(Json.JNum(1), Json.JNull)), "b" -> Json.JBool(true))).render)
  println(Json.JArr(Nil) == Json.JArr(List()))
  println(Json.JNull.toString + Json.JNum(1).toString)
