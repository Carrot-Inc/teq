// `TypeTest[S, T]` is not synthesized for a `T` at the bottom, nor for `AnyVal`, `AnyRef` and
// `Object` (Synthesizer.scala 97 and 105-106); the failure is reported with `TypeTest`'s
// `@implicitNotFound` text (MissingImplicitArgument.userDefinedImplicitNotFoundTypeMessage).
import scala.reflect.TypeTest
object Main:
  val a = summon[TypeTest[Any, AnyVal]]
  val b = summon[TypeTest[Any, Nothing]]
  val c = summon[TypeTest[Any, Null]]
  val d = summon[TypeTest[Any, AnyRef]]
  val e = summon[TypeTest[Int, Null]]
// expect: 6:40: error: No TypeTest available for [Any, AnyVal]
// expect: 7:41: error: No TypeTest available for [Any, Nothing]
// expect: 8:38: error: No TypeTest available for [Any, Null]
// expect: 9:40: error: No TypeTest available for [Any, AnyRef]
// expect: 10:38: error: No TypeTest available for [Int, Null]
