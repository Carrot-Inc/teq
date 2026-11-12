// teq: tests/interop/scalajs-stub
// dotty 3.8.4's tests/neg-scalajs/js-native-members.scala: a native JS type's members, constructors and private
// members (transformValOrDefDefInJSType, checkJSNativeDefinition), its diagnostics as scalac 3.8.4 -scalajs gives
// them on this file. An expectation's L:C is where scalac's caret begins (the start of dotty's span), 1-based.
// pending: teq is to report the
// 25 errors, and the six warnings
// with scalac's count line.
// expect: 80:10: warning: Ignoring [this] qualifier.
// expect: The syntax `private[this]` will be deprecated in the future; just write `private` instead.
// expect: See: https://docs.scala-lang.org/scala3/reference/dropped-features/this-qualifier.html
// expect: This construct can be rewritten automatically under -rewrite -source 3.4-migration.
// expect: 84:10: warning: Ignoring [this] qualifier.
// expect: The syntax `private[this]` will be deprecated in the future; just write `private` instead.
// expect: See: https://docs.scala-lang.org/scala3/reference/dropped-features/this-qualifier.html
// expect: This construct can be rewritten automatically under -rewrite -source 3.4-migration.
// expect: 88:10: warning: Ignoring [this] qualifier.
// expect: The syntax `private[this]` will be deprecated in the future; just write `private` instead.
// expect: See: https://docs.scala-lang.org/scala3/reference/dropped-features/this-qualifier.html
// expect: This construct can be rewritten automatically under -rewrite -source 3.4-migration.
// expect: 92:10: warning: Ignoring [this] qualifier.
// expect: The syntax `private[this]` will be deprecated in the future; just write `private` instead.
// expect: See: https://docs.scala-lang.org/scala3/reference/dropped-features/this-qualifier.html
// expect: This construct can be rewritten automatically under -rewrite -source 3.4-migration.
// expect: 105:19: warning: Ignoring [this] qualifier.
// expect: The syntax `private[this]` will be deprecated in the future; just write `private` instead.
// expect: See: https://docs.scala-lang.org/scala3/reference/dropped-features/this-qualifier.html
// expect: This construct can be rewritten automatically under -rewrite -source 3.4-migration.
// expect: 114:45: warning: with as a type operator has been deprecated; use & instead
// expect: This construct can be rewritten automatically under -rewrite -source 3.4-migration.
// expect: 65:25: error: A secondary constructor of a native JS class may only call the primary constructor
// expect: 67:16: error: Concrete members of JS native types may only call js.native.
// expect: 68:16: error: Concrete members of JS native types may only call js.native.
// expect: 69:16: error: Concrete members of JS native types may only call js.native.
// expect: 70:24: error: Concrete members of JS native types may only call js.native.
// expect: 73:24: error: Concrete members of JS native types may only call js.native.
// expect: 75:38: error: Concrete members of JS native types may only call js.native.
// expect: 81:15: error: Native JS classes may not have private constructors. Use `private[this]` to declare an internal constructor.
// expect: 82:19: error: Native JS classes may not have private constructors. Use `private[this]` to declare an internal constructor.
// expect: 84:3: error: Native JS classes may not have private members. Use a public member in a private facade instead.
// expect: 85:3: error: Native JS classes may not have private members. Use a public member in a private facade instead.
// expect: 86:3: error: Native JS classes may not have private members. Use a public member in a private facade instead.
// expect: 88:3: error: Native JS classes may not have private members. Use a public member in a private facade instead.
// expect: 89:3: error: Native JS classes may not have private members. Use a public member in a private facade instead.
// expect: 90:3: error: Native JS classes may not have private members. Use a public member in a private facade instead.
// expect: 92:3: error: Native JS classes may not have private members. Use a public member in a private facade instead.
// expect: 93:3: error: Native JS classes may not have private members. Use a public member in a private facade instead.
// expect: 94:3: error: Native JS classes may not have private members. Use a public member in a private facade instead.
// expect: 99:11: error: Native JS classes may not have private constructors. Use `private[this]` to declare an internal constructor.
// expect: 102:11: error: Native JS classes may not have private constructors. Use `private[this]` to declare an internal constructor.
// expect: 112:3: error: The type of foo must be explicitly specified because it is JS native.
// expect: 113:3: error: The type of bar must be explicitly specified because it is JS native.
// expect: 122:9: error: A member named apply represents function application in JavaScript. A parameterless member should be exported as a property. You must add @JSName("apply")
// expect: 133:9: error: A member named apply represents function application in JavaScript. A parameterless member should be exported as a property. You must add @JSName("apply")
// expect: 144:9: error: A member named apply represents function application in JavaScript. A parameterless member should be exported as a property. You must add @JSName("apply")
// expect: 6 warnings found
// expect: 25 errors found
import scala.scalajs.js
import scala.scalajs.js.annotation.*

// rhs must be `js.native` (or calling the primary constructor for constructors)

@js.native @JSGlobal
class A1(param: Int) extends js.Object {
  def this(param: Int, z: String) = this(param)
  def this(z: String) = this(z.length, z) // error

  val a: Int = 1 // error
  var b: Int = 2 // error
  def c: Int = 3 // error
  def d(x: Int): Int = x + 1 // error

  @JSBracketAccess
  def e(x: Any): Any = x // error
  @JSBracketCall
  def f(x: Any)(y: String): String = y // error
}

@js.native @JSGlobal
class A2 extends js.Object {
  private[this] def this(x: Int) = this() // ok
  private def this(x: String) = this() // error
  private[A2] def this(x: Boolean) = this() // error

  private[this] val a: Int = js.native // error
  private val b: Int = js.native // error
  private[A2] val c: Int = js.native // error

  private[this] var d: Int = js.native // error
  private var e: Int = js.native // error
  private[A2] var f: Int = js.native // error

  private[this] def g(): Int = js.native // error
  private def h(): Int = js.native // error
  private[A2] def i(): Int = js.native // error
}

object A3 {
  @js.native @JSGlobal("X")
  class X1 private () extends js.Object // error

  @js.native @JSGlobal("X")
  class X2 private[A3] () extends js.Object // error

  @js.native @JSGlobal("X")
  class X3 private[this] () extends js.Object { // ok
    def this(x: Int) = this()
  }
}

@js.native @JSGlobal
class A4 extends js.Object {
  def foo = js.native // error
  val bar = js.native // error
  def assign[T, U](target: T, source: U): T with U = js.native // ok
}

// Members named `apply`

object A5 {
  @js.native
  trait X1 extends js.Object {
    def apply: Int = js.native // error
  }

  @js.native
  trait X2 extends js.Object {
    @JSName("apply")
    def apply: Int = js.native // ok
  }

  @js.native
  trait X3 extends js.Object {
    val apply: Int = js.native // error
  }

  @js.native
  trait X4 extends js.Object {
    @JSName("apply")
    val apply: Int = js.native // ok
  }

  @js.native
  trait X5 extends js.Object {
    var apply: Int = js.native // error
  }

  @js.native
  trait X6 extends js.Object {
    @JSName("apply")
    var apply: Int = js.native // ok
  }
}
