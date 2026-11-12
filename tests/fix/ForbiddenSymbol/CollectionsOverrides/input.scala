/*
rule = ForbiddenSymbol
ForbiddenSymbol.symbols = [
  "scala.collection.IterableOps.init",
  "scala.collection.IterableOps.tail",
  "scala.collection.IterableOps.last",
  "fix.Base.own",
  "fix.Base.twice",
  "fix.Middle.twice"
]
*/
package fix

import scala.collection.mutable

// The program's own members: Base's two are entries, and Middle's override of one of them is.
trait Base:
  def own: Int = 0 // assert: ForbiddenSymbol
  def twice: Int = 0 // assert: ForbiddenSymbol

trait Middle extends Base:
  override def twice: Int = 1 // assert: ForbiddenSymbol

class Overriding extends Middle:
  override def own: Int = 2
  override def twice: Int = 2

class Inheriting extends Middle

// A program's class of a library's trait: `last` overridden in one, inherited by the other.
class OwnLast extends IndexedSeq[Int]:
  def apply(i: Int): Int = i
  def length: Int = 3
  override def last: Int = 0

class InheritedLast extends IndexedSeq[Int]:
  def apply(i: Int): Int = i
  def length: Int = 3

object CollectionsOverrides:
  val list = List(1, 2, 3)
  val vector = Vector(1, 2, 3)
  val indexedSeq: IndexedSeq[Int] = vector
  val range = 1 to 3
  val listBuffer = mutable.ListBuffer(1)
  val arrayBuffer = mutable.ArrayBuffer(1)
  val lazyList = LazyList(1)

  // List declares `last`; its `init` is IterableOps'.
  def listLast = list.last // assert: ForbiddenSymbol
  def listInit = list.init // assert: ForbiddenSymbol

  // Vector declares the three; as a Seq it selects IterableOps'.
  def vectorInit = vector.init // assert: ForbiddenSymbol
  def vectorTail = vector.tail // assert: ForbiddenSymbol
  def vectorLast = vector.last // assert: ForbiddenSymbol
  def vectorAsSeq = (vector: Seq[Int]).last // assert: ForbiddenSymbol

  // An IndexedSeq's `last` is IndexedSeqOps'.
  def indexedSeqLast = indexedSeq.last // assert: ForbiddenSymbol
  def indexedSeqInit = indexedSeq.init // assert: ForbiddenSymbol

  // Range declares the three.
  def rangeInit = range.init // assert: ForbiddenSymbol
  def rangeTail = range.tail // assert: ForbiddenSymbol
  def rangeLast = range.last // assert: ForbiddenSymbol
  def rangeAsSeq = (range: Seq[Int]).init // assert: ForbiddenSymbol

  // ListBuffer declares `last`; an ArrayBuffer's is IndexedSeqOps'.
  def listBufferLast = listBuffer.last // assert: ForbiddenSymbol
  def listBufferInit = listBuffer.init // assert: ForbiddenSymbol
  def arrayBufferLast = arrayBuffer.last // assert: ForbiddenSymbol
  def arrayBufferInit = arrayBuffer.init // assert: ForbiddenSymbol

  // LazyList declares `tail`; its `last` is LinearSeqOps'.
  def lazyListTail = lazyList.tail // assert: ForbiddenSymbol
  def lazyListLast = lazyList.last // assert: ForbiddenSymbol
  def lazyListInit = lazyList.init // assert: ForbiddenSymbol

  // Members that are no entry and override none.
  def heads = (list.head, vector.head, range.head, lazyList.head)

  def program(base: Base, middle: Middle, overriding: Overriding, inheriting: Inheriting): Int =
    base.own + // assert: ForbiddenSymbol
      base.twice + // assert: ForbiddenSymbol
      middle.own + // assert: ForbiddenSymbol
      middle.twice + // assert: ForbiddenSymbol
      overriding.own + // assert: ForbiddenSymbol
      overriding.twice + // assert: ForbiddenSymbol
      inheriting.own + // assert: ForbiddenSymbol
      inheriting.twice // assert: ForbiddenSymbol

  def library(ownLast: OwnLast, inheritedLast: InheritedLast): Int =
    ownLast.last + // assert: ForbiddenSymbol
      ownLast.init.size + // assert: ForbiddenSymbol
      inheritedLast.last + // assert: ForbiddenSymbol
      inheritedLast.init.size // assert: ForbiddenSymbol
