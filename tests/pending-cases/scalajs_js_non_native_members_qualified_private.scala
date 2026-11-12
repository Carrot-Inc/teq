// teq: tests/interop/scalajs-stub
// dotty 3.8.4's tests/neg-scalajs/js-non-native-members-qualified-private.scala: qualified-private members of
// non-native JS types (transformValOrDefDefInJSType), its diagnostics as scalac 3.8.4 -scalajs gives them on this
// file. An expectation's L:C is where scalac's caret begins (the start of dotty's span), 1-based.
// pending: teq is to report the
// 12 errors, and the warning
// with scalac's count line.
// expect: 77:22: warning: Ignoring [this] qualifier.
// expect: The syntax `private[this]` will be deprecated in the future; just write `private` instead.
// expect: See: https://docs.scala-lang.org/scala3/reference/dropped-features/this-qualifier.html
// expect: This construct can be rewritten automatically under -rewrite -source 3.4-migration.
// expect: 31:9: error: Qualified private members in non-native JS classes must be final
// expect: 32:9: error: Qualified private members in non-native JS classes must be final
// expect: 33:9: error: Qualified private members in non-native JS classes must be final
// expect: 45:9: error: Qualified private members in non-native JS classes must be final
// expect: 46:9: error: Qualified private members in non-native JS classes must be final
// expect: 47:9: error: Qualified private members in non-native JS classes must be final
// expect: 55:9: error: Qualified private members in non-native JS classes must be final
// expect: 56:9: error: Qualified private members in non-native JS classes must be final
// expect: 57:9: error: Qualified private members in non-native JS classes must be final
// expect: 69:9: error: Qualified private members in non-native JS classes must be final
// expect: 70:9: error: Qualified private members in non-native JS classes must be final
// expect: 71:9: error: Qualified private members in non-native JS classes must be final
// expect: 1 warning found
// expect: 12 errors found
import scala.scalajs.js
import scala.scalajs.js.annotation.*

    object Enclosing1 {
      class A extends js.Object {
        private[Enclosing1] def foo(i: Int): Int = i // error
        private[Enclosing1] val x: Int = 3 // error
        private[Enclosing1] var y: Int = 5 // error

        final private[Enclosing1] def babar(i: Int): Int = i // ok
      }

      class B extends A {
        override private[Enclosing1] final def foo(i: Int): Int = i + 1
      }
    }

    object Enclosing2 {
      object A extends js.Object {
        private[Enclosing2] def foo(i: Int): Int = i // error
        private[Enclosing2] val x: Int = 3 // error
        private[Enclosing2] var y: Int = 5 // error

        final private[Enclosing2] def babar(i: Int): Int = i // ok
      }
    }

    object Enclosing3 {
      abstract class A extends js.Object {
        private[Enclosing3] def foo(i: Int): Int // error
        private[Enclosing3] val x: Int // error
        private[Enclosing3] var y: Int // error

        final private[Enclosing3] def babar(i: Int): Int = i // ok
      }

      class B extends A {
        override private[Enclosing3] final def foo(i: Int): Int = i + 1
      }
    }

    object Enclosing4 {
      trait A extends js.Object {
        private[Enclosing4] def foo(i: Int): Int // error
        private[Enclosing4] val x: Int // error
        private[Enclosing4] var y: Int // error
      }
    }

    object Enclosing5 {
      class A private () extends js.Object // ok
      class B private[this] () extends js.Object // ok
      class C private[Enclosing5] () extends js.Object // ok
    }
