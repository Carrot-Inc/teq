package japgolly.scalajs.react.component

import japgolly.scalajs.react.vdom.{VdomElement, VdomNode}

final class JsFnComponentImpl[P]:
  def apply(props: P): VdomElement = ???
object JsFn:
  type Component[P, CT] = JsFnComponentImpl[P]
final class JsComponentImpl[P]:
  def apply(props: P): VdomElement = ???
object Js:
  type Component[P, S, CT] = JsComponentImpl[P]
final class JsForwardRefComponentImpl[P]:
  def apply(props: P): VdomElement = ???
object JsForwardRef:
  type Component[P, R, CT] = JsForwardRefComponentImpl[P]
