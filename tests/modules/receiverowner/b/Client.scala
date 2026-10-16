package rownb

import rowna.*

object Client:
  def direct(x: PublicChild): Int = x.method()
  def deep(x: GrandChild): Int = x.deep()
  def bounded[T <: PublicChild](x: T): Int = x.method()
  def intersection(x: Serializable & PublicChild): Int = x.method()
  def opaque(x: Library.Wrapped): Int = x.method()
  def inlined(x: PublicChild): Int = Library.inlineMethod(x)
  def inlineBound(x: GrandChild): Int = Library.inlineBound(x)
  def read(x: PublicChild): Int = x.field
  def write(x: PublicChild): Int = { x.field = 9; x.field }
  def default(x: GrandChild): Int = x.withDefault()
  def cloned(x: CloneChild): Boolean = x.clone() eq x
  def inherited(): Int = Inherited.method() + Inherited.deep()
  def plain(x: GrandChild): Boolean = x.toString.contains("GrandChild@")

  def main(args: Array[String]): Unit =
    val c = Library.child()
    val g = new GrandChild()
    println(List(direct(c), deep(g), bounded(g), opaque(Library.wrap(c)), inlined(c), inlineBound(g)))
    println(List(read(c), write(c), default(g), cloned(new Cloned()), inherited(), plain(g)))
