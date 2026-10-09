// Tuples of more than 22 elements through the generic operations: `Tuple.fromArray` past 22 is
// scalac's `scala.runtime.TupleXXL`, an array-backed product, and every operation on a tuple
// whose length its type does not give keeps scalac's classes at the boundaries (a cons onto a
// `Tuple22` and a tail of 24 elements are a TupleXXL, a tail of 23 a `Tuple22`), its text,
// equality and hash. Last, the derivation shape of a JSON decoder: the fields of a 34-field case
// class decoded into an array, `Tuple.fromArray`, then the mirror's `fromProduct`.
import scala.compiletime.{constValue, erasedValue, summonInline}
import scala.deriving.Mirror

def cls(x: Any): String = x.getClass.getName
def show(label: String, t: Tuple): Unit = println(label + ": " + cls(t) + " " + t.size + " " + t)
def nums(n: Int, from: Int = 0): Tuple = Tuple.fromArray(Array.tabulate[Any](n)(i => from + i))
def strs(n: Int): Tuple = Tuple.fromArray(Array.tabulate[Any](n)(i => "e" + i))

trait Dec[A]:
  def dec(s: String): A
object Dec:
  given Dec[Int] = _.toInt
  given Dec[Long] = _.toLong
  given Dec[String] = s => s
  given Dec[Boolean] = _ == "true"
  given Dec[Double] = _.toDouble
  given Dec[Option[String]] = s => if s.isEmpty then None else Some(s)

// Two elements a step, which keeps 34 fields under scalac's 32 successive inlines.
inline def labels[T <: Tuple]: List[String] = inline erasedValue[T] match
  case _: EmptyTuple => Nil
  case _: (h1 *: h2 *: t) => constValue[h1].asInstanceOf[String] :: constValue[h2].asInstanceOf[String] :: labels[t]
  case _: (h *: t) => constValue[h].asInstanceOf[String] :: labels[t]
inline def decs[T <: Tuple]: List[Dec[?]] = inline erasedValue[T] match
  case _: EmptyTuple => Nil
  case _: (h1 *: h2 *: t) => summonInline[Dec[h1]] :: summonInline[Dec[h2]] :: decs[t]
  case _: (h *: t) => summonInline[Dec[h]] :: decs[t]

final class ObjDec[A](names: List[String], ds: List[Dec[?]], m: Mirror.ProductOf[A]):
  def dec(fields: Map[String, String]): A =
    val values = new Array[Any](names.length)
    var i = 0
    names.zip(ds).foreach { (n, d) =>
      values(i) = d.dec(fields(n))
      i += 1
    }
    m.fromProduct(Tuple.fromArray(values))
inline def derive[A](using m: Mirror.ProductOf[A]): ObjDec[A] =
  ObjDec(labels[m.MirroredElemLabels], decs[m.MirroredElemTypes], m)

case class Hold(
    id: Int, name: String, placed: Long, paid: Boolean, total: Double, note: Option[String],
    branch: Int, city: String, slot: Long, ready: Boolean, weight: Double, door: Option[String],
    items: Int, phone: String, eta: Long, fragile: Boolean, fine: Double, waiver: Option[String],
    lane: Int, clerk: String, updated: Long, cold: Boolean, tax: Double, gate: Option[String],
    bags: Int, zone: String, created: Long, gift: Boolean, fee: Double, floor: Option[String],
    count: Int, region: String, version: Long, archived: Boolean)

@main def run(): Unit =
  val t23 = nums(23)
  val t24 = nums(24, 100)
  val t22 = strs(22)
  show("fromArray 23", t23)
  show("fromArray 34", strs(34))
  show("fromArray 22", t22)
  println(t23(0).toString + " " + t23(22) + " " + t23.head + " " + t23.last)

  // The boundaries: 22 -> 23 by cons, 23 -> 22 and 24 -> 23 by tail.
  show("cons onto 22", "x" *: t22)
  show("cons onto 23", "y" *: t23)
  show("tail of 23", t23.tail)
  show("tail of 24", t24.tail)
  show("tail of tail of 24", t24.tail.tail)
  show("init of 23", t23.init)
  show("append to 22", t22 :* "z")

  // Concatenation, zip, map and reverse across 22.
  show("12 ++ 11", nums(12) ++ nums(11, 50))
  show("22 ++ 2", t22 ++ nums(2))
  show("23 ++ empty", t23 ++ EmptyTuple)
  show("empty ++ 23", EmptyTuple ++ t23)
  show("zip 23 24", t23.zip(t24))
  show("zip 23 22", t23.zip(t22))
  show("map 23", t23.map([t] => (x: t) => List(x)))
  show("reverse 23", t23.reverse)
  show("reverse 22", t22.reverse)

  // take, drop and splitAt on either side of 22.
  show("take 22 of 24", t24.take(22))
  show("take 23 of 24", t24.take(23))
  show("take 30 of 24", t24.take(30))
  show("drop 1 of 24", t24.drop(1))
  show("drop 2 of 24", t24.drop(2))
  show("drop 0 of 23", t23.drop(0))
  val (l, r) = t24.splitAt(1)
  show("splitAt 1 left", l)
  show("splitAt 1 right", r)
  val (l2, r2) = t23.splitAt(22)
  show("splitAt 22 left", l2)
  show("splitAt 22 right", r2)

  // The elements out: arrays, lists and the product's members.
  println(t23.toArray.length.toString + " " + t23.toArray.mkString("[", " ", "]"))
  println(t23.toList)
  println(t23.toIArray.length)
  val p = t23.asInstanceOf[Product]
  val xxl = t23.asInstanceOf[scala.runtime.TupleXXL]
  println(p.productArity.toString + " " + xxl.productPrefix + " [" + p.productElementName(3) + "] " + p.productElementNames.size)
  println(p.productIterator.toList.takeRight(3))
  try println(p.productElement(23))
  catch case e: IndexOutOfBoundsException => println(cls(e) + ": " + e.getMessage)

  // Equality and hash: separately made equal tuples, other lengths, a changed last element, and
  // 1 against 1L, which are equal with one hash.
  val again = nums(23)
  println(t23 == again)
  println(t23.hashCode == again.hashCode)
  println(t23.hashCode)
  println(strs(34).hashCode)
  println(t23 == nums(24))
  println(t23 == Tuple.fromArray(Array.tabulate[Any](23)(i => if i == 22 then -1 else i)))
  val ints = Tuple.fromArray(Array.tabulate[Any](23)(i => if i == 0 then 1 else i))
  val longs = Tuple.fromArray(Array.tabulate[Any](23)(i => if i == 0 then 1L else i))
  println(ints == longs)
  println(ints.hashCode.toString + " " + longs.hashCode)
  println(xxl.canEqual(again).toString + " " + xxl.canEqual(t24) + " " + xxl.canEqual(t22))
  println(t23 == t22)
  println(t23.toString == again.toString)

  // Who owns the array: `fromArray` and `toArray` copy, `fromIArray` and `toIArray` share, and
  // `fromProduct` of a tuple is that tuple.
  val src = Array.tabulate[Any](23)(i => i)
  val owned = Tuple.fromArray(src)
  src(0) = "changed"
  val out = owned.toArray
  out(1) = "changed"
  println(owned(0).toString + " " + owned(1))
  val shared = Tuple.fromIArray(IArray.tabulate[Any](25)(i => i))
  println(shared.toIArray.asInstanceOf[AnyRef] eq shared.toIArray.asInstanceOf[AnyRef])
  println(Tuple.fromProduct(p).asInstanceOf[AnyRef] eq p.asInstanceOf[AnyRef])
  show("fromProduct of a 23 class", Tuple.fromProduct(p))

  // A type test and a cons pattern take a TupleXXL.
  println(t23.isInstanceOf[NonEmptyTuple])
  (t24: Any) match
    case h *: rest => println("cons pattern: " + h + " " + cls(rest))
    case _ => println("no match")

  // The decoder's shape: a 34-field case class through `Tuple.fromArray` and the mirror.
  val input = Map(
    "id" -> "7", "name" -> "Ada", "placed" -> "10000000001", "paid" -> "true", "total" -> "12.5", "note" -> "",
    "branch" -> "3", "city" -> "Oslo", "slot" -> "10000000002", "ready" -> "false", "weight" -> "1.25", "door" -> "B",
    "items" -> "4", "phone" -> "555", "eta" -> "10000000003", "fragile" -> "true", "fine" -> "2.25", "waiver" -> "",
    "lane" -> "9", "clerk" -> "Bo", "updated" -> "10000000004", "cold" -> "true", "tax" -> "0.5", "gate" -> "G2",
    "bags" -> "2", "zone" -> "N", "created" -> "10000000005", "gift" -> "false", "fee" -> "1.5", "floor" -> "3",
    "count" -> "11", "region" -> "EU", "version" -> "10000000006", "archived" -> "false")
  val hold = derive[Hold].dec(input)
  println(hold)
  println(hold.productArity.toString + " " + hold.region + " " + hold.archived)
  val fields = Tuple.fromProduct(hold)
  show("fields", fields)
  println(summon[Mirror.ProductOf[Hold]].fromProduct(fields) == hold)
