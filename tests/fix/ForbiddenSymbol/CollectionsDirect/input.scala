/*
rule = ForbiddenSymbol
ForbiddenSymbol.symbols = [
  "scala.collection.IterableOps.init",
  "scala.collection.IterableOps.tail",
  "scala.collection.IterableOps.last",
  "scala.collection.ArrayOps.init",
  "scala.collection.ArrayOps.head",
  "scala.collection.StringOps.init",
  "scala.collection.StringOps.last",
  "scala.collection.IterableOnce.min",
  "scala.collection.IterableOnce.minBy",
  "scala.collection.IterableOnce.max",
  "scala.collection.IterableOnce.maxBy",
  "scala.collection.IterableOnce.reduce"
]
*/
package fix

object CollectionsDirect:
  val list = List(1, 2, 3)
  val seq: Seq[Int] = list
  val set = Set(1, 2)
  val map = Map(1 -> 2)
  val iterable: Iterable[Int] = list
  val array = Array(1, 2, 3)
  val str = "abc"
  val nested: Seq[Seq[Int]] = Seq(seq, seq)
  val arrays = Array(array, array)
  val strs: Seq[String] = Seq("ab", "cd")
  val fns: Seq[Int => Int] = Seq(_ + 1)

  // IterableOps' own members, selected through a Seq, a Set, an Iterable, a Map and a List.
  def seqInit = seq.init // assert: ForbiddenSymbol
  def seqTail = seq.tail // assert: ForbiddenSymbol
  def seqLast = seq.last // assert: ForbiddenSymbol
  def seqHead = seq.head
  def setInit = set.init // assert: ForbiddenSymbol
  def setTail = set.tail // assert: ForbiddenSymbol
  def setLast = set.last // assert: ForbiddenSymbol
  def iterableInit = iterable.init // assert: ForbiddenSymbol
  def iterableTail = iterable.tail // assert: ForbiddenSymbol
  def iterableLast = iterable.last // assert: ForbiddenSymbol
  def mapInit = map.init // assert: ForbiddenSymbol
  def listInit = list.init // assert: ForbiddenSymbol
  def listTail = list.tail // assert: ForbiddenSymbol
  def listHead = list.head

  // ArrayOps' and StringOps' members, reached by a conversion.
  def arrayInit = array.init // assert: ForbiddenSymbol
  def arrayHead = array.head // assert: ForbiddenSymbol
  def arrayTail = array.tail
  def arrayLast = array.last
  def strInit = str.init // assert: ForbiddenSymbol
  def strLast = str.last // assert: ForbiddenSymbol
  def strTail = str.tail
  def strHead = str.head

  // A member reached and then applied, mapped, iterated or matched.
  def applied = nested.tail(0) // assert: ForbiddenSymbol
  def appliedInit = nested.init(0) // assert: ForbiddenSymbol
  def appliedArray = arrays.head(0) // assert: ForbiddenSymbol
  def appliedArrayInit = arrays.init(0) // assert: ForbiddenSymbol
  def appliedStr = strs.init(0) // assert: ForbiddenSymbol
  def appliedChar = strs.head.init(0) // assert: ForbiddenSymbol
  def appliedTwice = fns.tail(0)(1) // assert: ForbiddenSymbol
  def appliedByName = nested.tail.apply(0) // assert: ForbiddenSymbol
  def mapped = nested.init.map(_.last) // assert: ForbiddenSymbol
  def foreached = nested.init.foreach(println) // assert: ForbiddenSymbol
  def iterated = for xs <- nested.init yield xs.last // assert: ForbiddenSymbol
  def matched = nested.init match // assert: ForbiddenSymbol
    case xs +: _ => xs.last // assert: ForbiddenSymbol
    case _ => 0
  def mappedStr = strs.map(_.init).mkString // assert: ForbiddenSymbol
  def flatMapped = strs.flatMap(_.init) // assert: ForbiddenSymbol

  // A backquoted name, an operand of an infix operator, a receiver in parentheses, an interpolated string.
  def backquoted = seq.`last` // assert: ForbiddenSymbol
  def operands = seq.init ++ seq.tail // assert: ForbiddenSymbol
  def parenthesized = (seq).last // assert: ForbiddenSymbol
  def interpolated = s"${seq.last} ${str.last}" // assert: ForbiddenSymbol
  def chained = seq.tail.tail.init // assert: ForbiddenSymbol

  // min, minBy, max, maxBy and reduce: IterableOnceOps declares them, IterableOnce does not.
  def mins = (seq.min, set.min, array.min, str.min)
  def maxs = (seq.max, list.max, iterable.max)
  def minBys = (seq.minBy(identity), map.minBy(_._1))
  def maxBys = (list.maxBy(identity), array.maxBy(identity))
  def reduced = (seq.reduce(_ + _), set.reduce(_ + _), array.reduce(_ + _))
  def reducedInfix = seq reduce (_ + _)
