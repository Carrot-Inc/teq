package fix.members

trait Container:
  type Elem
  type Alias = List[Int]
  opaque type Id = Long
  class Inner
  def head: Elem
  def viaAlias: Alias
  def viaOpaque: Id
  def viaInner: Inner
  def put(e: Elem): Unit
  def same(other: Container): other.Elem
  def project(x: Container#Elem): Int

object Holder:
  type Abstractish >: Nothing <: Any
  def plain(xs: List[Int]): Option[String] = None
  def dep(c: Container)(e: c.Elem): c.Elem = e

object Registry:
  class Entry
  type Key = String
  def make(k: Registry.Key): Registry.Entry = new Entry

object UsesRegistry:
  def lookup(k: Registry.Key): Registry.Entry = Registry.make(k)
