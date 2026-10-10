package oia

trait Named { def name: String; def greet: String = s"hello $name" }
trait Sized { def size: Int; def name: String }
class Item(val name: String, val size: Int) extends Named with Sized

trait Source[T] { def items: List[T] }
trait Counted { def count: Int }
class Bag[T](val items: List[T]) extends Source[T] with Counted { def count = items.length }

object Store:
  opaque type Handle <: Named & Sized = Item
  def handle(name: String, size: Int): Handle = Item(name, size)
  opaque type Pack[T] <: Source[T] & Counted = Bag[T]
  def pack[T](xs: T*): Pack[T] = Bag(xs.toList)
