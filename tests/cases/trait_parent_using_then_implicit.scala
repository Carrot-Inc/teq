// A trait whose using clause is followed by an implicit one takes no empty clause after them (NamerOps'
// normalizeIfConstructor adds it after using clauses alone): applied without a `()`, its omitted parameters are
// searched (the review's queue 2), a short using list given to the implicit clause leaves the rest to the search, and an
// empty list given to it takes its defaults.
class A(val s: String)
class B(val s: String)
class C(val s: String)
trait T(using a: A)(implicit b: B, c: C):
  def show = a.s + b.s + c.s
trait TV(using a: A)(implicit b: B = B("db"), c: C = C("dc")):
  def show = a.s + b.s + c.s
object WithGivens:
  given B = B("gb")
  given C = C("gc")
  class Named extends T(using A("wa"))(using c = C("wc"))
  class Short extends T(using A("wa"))(using B("wb"))
  class Omitted extends T(using A("wa"))
  def run(): Unit =
    println(new Named().show)
    println(new Short().show)
    println(new Omitted().show)
    println((new T(using A("wa"))(using c = C("wc")) {}).show)
object NoGivens:
  class Defaults extends TV(using A("wa"))()
  def run(): Unit =
    println(new Defaults().show)
    println((new TV(using A("wa"))() {}).show)
object Main:
  def main(args: Array[String]): Unit =
    WithGivens.run()
    NoGivens.run()
