trait Enclosure:
  sealed abstract class Item[+A](val value: Int)
  object Internal:
    case class End(override val value: Int) extends Item[Nothing](value)
  val alias: Enclosure.this.type = Enclosure.this
  def factory(value: Int): Item[Nothing] = Internal.End(value)
  def selected(value: alias.Internal.End): alias.Item[Nothing] = value

def localClass(): Int =
  class Cell[A](val value: A)
  def retain[A](cell: Cell[A]): Cell[A] = cell
  retain(new Cell(9)).value

@main def appliedInnerClassPrefix(): Unit =
  val enclosure = new Enclosure {}
  println(enclosure.factory(5).value)
  val direct: enclosure.Item[Nothing] = new enclosure.Internal.End(6)
  println(direct.value)
  val held = enclosure.Internal.End(7)
  val widened: enclosure.Item[Nothing] = held
  println(widened.value)
  println(enclosure.selected(enclosure.alias.Internal.End(8)).value)
  println(localClass())
