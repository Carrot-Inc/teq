// A macro's trees of a class's members as dotty's typer leaves them: an `Ident` where
// `prefixIsElidable` holds (a parameter accessor read in its class, an object's member), a
// `Select(This(Some(C)), name)` otherwise (a method, a member read from a nested class); shown as
// Printer.TreeCode shows them, by the ThisType prefix (`Holder.this.name`) or the object's path.
package shown

class Holder(val name: String):
  def size: Int = name.length
  def shownName: String = Shown.show(name)
  def shownCall: String = Shown.show(size + 1)
  def shownThis: String = Shown.show(this)
  def nameStructure: String = Shown.structure(name)
  def sizeStructure: String = Shown.structure(size)
  class Inside:
    def shown: String = Shown.show(name)
    def shownCall: String = Shown.show(size * 2)
    def shownOuter: String = Shown.show(Holder.this)
    def outerStructure: String = Shown.structure(Holder.this)
    def nameStructure: String = Shown.structure(name)

object Config:
  val limit = 3
  def shown: String = Shown.show(limit * 2)

@main def run(): Unit =
  val h = new Holder("n")
  println(h.shownName)
  println(h.shownCall)
  println(h.shownThis)
  println(h.nameStructure)
  println(h.sizeStructure)
  val i = new h.Inside()
  println(i.shown)
  println(i.shownCall)
  println(i.shownOuter)
  println(i.outerStructure)
  println(i.nameStructure)
  println(Config.shown)
