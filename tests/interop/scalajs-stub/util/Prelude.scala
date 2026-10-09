// What the facade files take from the application's prelude.
package util

import japgolly.scalajs.react.vdom

object Prelude:
  export vdom.{VdomElement, VdomNode}

  object Attr:
    trait ValueType[A, B]
    object ValueType:
      def apply[A, B](fn: (B => Unit, A) => Unit): ValueType[A, B] = new ValueType[A, B] {}

  extension [A](a: A)
    def ===(b: A): Boolean = a == b
