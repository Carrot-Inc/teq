// The shapes of scalajs-react that the facade files name; the constructor type parameter of a
// component is carried along without a meaning of its own.
package japgolly.scalajs.react

import scala.scalajs.js

object Children:
  trait None
  trait Varargs

object CtorType:
  trait Props
  trait PropsAndChildren

trait ReactMouseEvent

object JsFnComponent:
  def apply[P, C](raw: js.Any): component.JsFnComponentImpl[P] = ???
  def force[P, C](raw: js.Any): component.JsFnComponentImpl[P] = ???

object JsComponent:
  def apply[P, C, S](raw: js.Any): component.JsComponentImpl[P] = ???

object JsForwardRefComponent:
  def force[P, C, R](raw: js.Any): component.JsForwardRefComponentImpl[P] = ???
