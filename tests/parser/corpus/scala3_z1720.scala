// Adapted from scala3 tests/pos/z1720.scala (Apache-2.0, see tests/scala3/README.md): a method
// whose result names `this.type`, passed to one with a dependent result `Info[t.type]`; the
// package clause is dropped and the prints are added.
class Thing:
  def info: Info[this.type] = InfoRepository.getInfo(this)
  def info2: Info[this.type] =
    def self: this.type = this
    InfoRepository.getInfo(self)

trait Info[T]
case class InfoImpl[T](thing: T) extends Info[T]

object InfoRepository:
  def getInfo(t: Thing): Info[t.type] = InfoImpl(t)

@main def main(): Unit =
  val t = Thing()
  val i: Info[t.type] = t.info
  println(i == InfoImpl(t))
  println(t.info2 == t.info)
