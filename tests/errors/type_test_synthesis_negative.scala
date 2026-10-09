// `TypeTest[S, T]` is not synthesized for a `T` at the bottom, nor for `AnyVal`, `AnyRef` and
// `Object` (Synthesizer.scala 97 and 105-106).
import scala.reflect.TypeTest
object Main:
  val a = summon[TypeTest[Any, AnyVal]]
  val b = summon[TypeTest[Any, Nothing]]
  val c = summon[TypeTest[Any, Null]]
  val d = summon[TypeTest[Any, AnyRef]]
  val e = summon[TypeTest[Int, Null]]
// expect: 5:40: error: no given instance of type TypeTest[Any, AnyVal] was found for parameter x
// expect: 6:41: error: no given instance of type TypeTest[Any, Nothing] was found for parameter x
// expect: 7:38: error: no given instance of type TypeTest[Any, Null] was found for parameter x
// expect: 8:40: error: no given instance of type TypeTest[Any, AnyRef] was found for parameter x
// expect: 9:38: error: no given instance of type TypeTest[Int, Null] was found for parameter x
