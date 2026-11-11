// Every function type past 22 parameters erases to the one class `FunctionXXL` (`Definitions.functionTypeErasure`,
// `FunctionXXLClass.typeRef`): a type test of any such type takes every function past 22 parameters, a cast from one
// to another is the value itself, and a cast of anything else to one fails naming `scala.runtime.FunctionXXL`.
package function_xxl_erased_class

type F23 = (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int) => Int
type F25 = (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int) => Int
type F2 = (Int, Int) => Int

def kind(a: Any): String = a match
  case _: F2 @unchecked => "f2"
  case _: F25 @unchecked => "xxl"
  case _ => "other"

@main def main(): Unit =
  val f23: F23 = (a0: Int, a1: Int, a2: Int, a3: Int, a4: Int, a5: Int, a6: Int, a7: Int, a8: Int, a9: Int, a10: Int, a11: Int, a12: Int, a13: Int, a14: Int, a15: Int, a16: Int, a17: Int, a18: Int, a19: Int, a20: Int, a21: Int, a22: Int) => a0 + 23
  val f25: F25 = (a0: Int, a1: Int, a2: Int, a3: Int, a4: Int, a5: Int, a6: Int, a7: Int, a8: Int, a9: Int, a10: Int, a11: Int, a12: Int, a13: Int, a14: Int, a15: Int, a16: Int, a17: Int, a18: Int, a19: Int, a20: Int, a21: Int, a22: Int, a23: Int, a24: Int) => a0 + 25
  val f2: F2 = (a, b) => a + b
  println(List(kind(f23), kind(f25), kind(f2), kind("s")))
  println((f23: Any).isInstanceOf[F25 @unchecked])
  println((f23: Any).isInstanceOf[scala.Function023[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] @unchecked])
  println(f23.asInstanceOf[F25].asInstanceOf[AnyRef] eq f23.asInstanceOf[AnyRef])
  println((f25: Any).asInstanceOf[F23].asInstanceOf[AnyRef] eq f25.asInstanceOf[AnyRef])
  try println(("x": Any).asInstanceOf[F25])
  catch case e: ClassCastException => println(e.getMessage.contains("scala.runtime.FunctionXXL"))
