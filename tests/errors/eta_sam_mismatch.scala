// expect: 13:44: error: type mismatch: found Builder, required TagMod => TagMod
// expect: 15:24: error: type mismatch: found Builder, required Int
// A method eta-expanded into a trait takes the trait method's parameters: an extension whose
// parameter is a function does not implement `applyTo(b: Builder)`, as under scalac.
class Builder
trait TagMod:
  def applyTo(b: Builder): Unit
object Prelude:
  extension [A](as: Seq[A])
    def toTagMod(f: A => TagMod): TagMod = ???
import Prelude.*
object Main:
  def render(props: Seq[TagMod]): TagMod = props.toTagMod
  def byInt(i: Int): Unit = ()
  val direct: TagMod = byInt
