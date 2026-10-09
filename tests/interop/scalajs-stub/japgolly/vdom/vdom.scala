package japgolly.scalajs.react.vdom

import scala.scalajs.js

trait VdomNode
// Children are applied to the element, which stands for the two call forms of a component.
trait VdomElement extends VdomNode:
  def apply(children: VdomNode*): VdomElement = this
trait TagMod:
  def toJs: TagModJs
trait TagModJs:
  def props: js.Object
  def addClassNameToProps(): Unit
  def addStyleToProps(): Unit
object all:
  object style:
    def :=(value: js.Any): TagMod = ???
  extension (mods: Seq[TagMod])
    def toTagMod: TagMod = ???
