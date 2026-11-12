// teq: tests/interop/scalajs-stub
// dotty 3.8.4's tests/neg-scalajs/js-type-override-equals-hashcode.scala: equals and hashCode overridden in JS types
// (transformValOrDefDefInJSType), its errors as scalac 3.8.4 -scalajs gives them on this file. An expectation's L:C
// is where scalac's caret begins (the start of dotty's span), 1-based.
// pending: teq is to accept
// `override def finalize(): Unit` (scalac's
// java.lang.Object has finalize) and report the eight.
// expect: 37:16: error: error overriding method hashCode(): Int in a JS class;
// expect: method hashCode(): Int is considered final in trait js.Any;
// expect: if you want to define a method named "hashCode" in JavaScript, use a different name and add @JSName("hashCode").
// expect: 38:16: error: error overriding method equals(that: Any): Boolean in a JS class;
// expect: method equals(that: Any): Boolean is considered final in trait js.Any;
// expect: if you want to define a method named "equals" in JavaScript, use a different name and add @JSName("equals").
// expect: 56:16: error: error overriding method hashCode(): Int in a JS class;
// expect: method hashCode(): Int is considered final in trait js.Any;
// expect: if you want to define a method named "hashCode" in JavaScript, use a different name and add @JSName("hashCode").
// expect: 57:16: error: error overriding method equals(that: Any): Boolean in a JS class;
// expect: method equals(that: Any): Boolean is considered final in trait js.Any;
// expect: if you want to define a method named "equals" in JavaScript, use a different name and add @JSName("equals").
// expect: 62:16: error: error overriding method hashCode(): Int in a JS class;
// expect: method hashCode(): Int is considered final in trait js.Any;
// expect: if you want to define a method named "hashCode" in JavaScript, use a different name and add @JSName("hashCode").
// expect: 63:16: error: error overriding method equals(that: Any): Boolean in a JS class;
// expect: method equals(that: Any): Boolean is considered final in trait js.Any;
// expect: if you want to define a method named "equals" in JavaScript, use a different name and add @JSName("equals").
// expect: 67:16: error: error overriding method hashCode(): Int in a JS class;
// expect: method hashCode(): Int is considered final in trait js.Any;
// expect: if you want to define a method named "hashCode" in JavaScript, use a different name and add @JSName("hashCode").
// expect: 68:16: error: error overriding method equals(that: Any): Boolean in a JS class;
// expect: method equals(that: Any): Boolean is considered final in trait js.Any;
// expect: if you want to define a method named "equals" in JavaScript, use a different name and add @JSName("equals").
// expect: 8 errors found
import scala.scalajs.js
import scala.scalajs.js.annotation.*

class A extends js.Object {
  override def hashCode(): Int = 1 // error
  override def equals(obj: Any): Boolean = false // error

  // this one works as expected (so allowed)
  override def toString(): String = "frobber"

  /* these are allowed, since they are protected in jl.Object.
   * as a result, only the overrides can be called. So the fact that they
   * do not truly override the methods in jl.Object is not observable.
   */
  override def clone(): Object = null
  override def finalize(): Unit = ()

  // other methods in jl.Object are final.
}

@js.native
@JSGlobal
object B extends js.Object {
  override def hashCode(): Int = js.native // error
  override def equals(obj: Any): Boolean = js.native // error
}

@js.native
trait C extends js.Any {
  override def hashCode(): Int = js.native // error
  override def equals(obj: Any): Boolean = js.native // error
}

trait D extends js.Any {
  override def hashCode(): Int // error
  override def equals(obj: Any): Boolean // error
}
