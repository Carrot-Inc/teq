package cfb

import cfa.*
import scala.annotation.nowarn

@main def run(): Unit =
  import cfa.Lib.*
  attempt("F1 nested") { array(new Array[Array[Nothing]](0)) }
  attempt("F1 inline") { arrayInline(new Array[Array[Nothing]](0)) }
  attempt("F2 bound product") { bound[Tuple, (Int, Int)](Record(1)) }
  attempt("F2 inline tuple") { boundInline[Tuple, Tuple](Record(1)) }
  attempt("F2 array generic") { arrayBound[Int]("text") }
  attempt("F2 array bound") { arrayRefBound[(Int, Int)]("text") }
  attempt("F2 chain right") { println(head((2, 3))) }
  attempt("F2 chain wrong") { head(((2, 3, 4): Any).asInstanceOf[Int *: Int *: EmptyTuple]) }
  attempt("F2 chain inline wrong") { headInline(((2, 3, 4): Any).asInstanceOf[Int *: Int *: EmptyTuple]) }
  failing()
  attempt("F3 Short-Double") { println(primitive[Short, Double](4.toShort).toInt) }
  attempt("F4 bound") { println(boxed[scala.runtime.BoxedUnit]("text") == null) }
  attempt("F4 inline") { println(boxedInline[scala.runtime.BoxedUnit](null) == null) }
  attempt("F4 static null") { println(nullBox == null) }; println(effects)
  attempt("F5 inherited unit") { inherited(()) }
  attempt("F5 inline unit") { inheritedInline(()) }
  attempt("F5 null") { inherited(null) }
  attempt("F5 good") { inherited(new Good) }
  attempt("F6 inherited") { println(serial(new Derived(2))) }
  attempt("F6 inline inherited") { println(serialInline(new Derived(3))) }
  attempt("F6 object") { println(serial(Marker)) }
  attempt("F6 null") { println(serial(null)) }
  attempt("F6 negative") { serial(new Good) }
  println(accept(new Derived(4)))

// scalac warns that these conversions always fail where it expands them, which teq does not.
@nowarn("msg=will always fail at runtime")
def failing(): Unit =
  import cfa.Lib.*
  attempt("F3 Boolean-Char") { primitive[Boolean, Char](flag); () }; println(effects)
  attempt("F3 Unit-Boolean") { primitive[Unit, Boolean](unit); () }; println(effects)
