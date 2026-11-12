package fxl.use

import fxl.*
import scala.runtime.FunctionXXL

// The caller of tests/function-xxl.sh, which scalac 3.8.4 is to build over teq's callee and teq over
// scalac's, run over either compiler's callee classes: every call passes, returns or applies a
// FunctionXXL, which both compilers' class files must agree on; a call that links to nothing or
// casts to the wrong class prints its error's class in place of its result.
object Main:
  def attempt(name: String)(f: => Any): Unit =
    try println(name + ": " + f)
    catch
      case e: LinkageError => println(name + ": " + e.getClass.getName)
      case e: ClassCastException => println(name + ": " + e.getClass.getName)

  def main(args: Array[String]): Unit =
    val c = new Callee
    attempt("id23(null)")(c.id23(null))
    attempt("id023(null)")(c.id023(null))
    attempt("run(lambda)")(c.run((a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23) => a1 + a2 + a3 + a4 + a5 + a6 + a7 + a8 + a9 + a10 + a11 + a12 + a13 + a14 + a15 + a16 + a17 + a18 + a19 + a20 + a21 + a22 + a23))
    attempt("mk(1..23)")(c.mk(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))
    attempt("run(mk)")(c.run(c.mk))
    attempt("id23(lambda)")(c.id23((a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23) => a1 + a2 + a3 + a4 + a5 + a6 + a7 + a8 + a9 + a10 + a11 + a12 + a13 + a14 + a15 + a16 + a17 + a18 + a19 + a20 + a21 + a22 + a23)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))
    attempt("runSpelled(lambda)")(c.runSpelled((a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23) => a1 + a2 + a3 + a4 + a5 + a6 + a7 + a8 + a9 + a10 + a11 + a12 + a13 + a14 + a15 + a16 + a17 + a18 + a19 + a20 + a21 + a22 + a23))
    attempt("run64(mk64)")(c.run64(c.mk64))
    attempt("mk64(1..64)")(c.mk64(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64))
    attempt("runContext")(c.runContext((a1: Int, a2: Int, a3: Int, a4: Int, a5: Int, a6: Int, a7: Int, a8: Int, a9: Int, a10: Int, a11: Int, a12: Int, a13: Int, a14: Int, a15: Int, a16: Int, a17: Int, a18: Int, a19: Int, a20: Int, a21: Int, a22: Int, a23: Int) ?=> a1 * 100 + a23))
    attempt("Sum23C via Function23")({ val g: Function23[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] = new Sum23C; g(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23) })
    attempt("run(new Sum23C)")(c.run(new Sum23C))
    attempt("run(Last23)")(c.run(Last23))
    attempt("FunctionErasure.id(null)")(new FunctionErasure().id(null))
    val xs = IArray.unsafeFromArray(Array.tabulate[Object](23)(i => Integer.valueOf(i + 1)))
    attempt("mk is a FunctionXXL")(c.mk.isInstanceOf[FunctionXXL])
    attempt("mk.apply(xs)")(c.mk.asInstanceOf[FunctionXXL].apply(xs))
    attempt("Sum23C is a FunctionXXL")((new Sum23C: Any).isInstanceOf[FunctionXXL])
    attempt("Sum23C.apply(xs)")((new Sum23C: Any).asInstanceOf[FunctionXXL].apply(xs))
    attempt("Sum23C.toString")(new Sum23C().toString)
    attempt("Last23.apply(xs)")((Last23: Any).asInstanceOf[FunctionXXL].apply(xs))
    attempt("Overloaded23 via Function23")({ val g: Function23[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] = new Overloaded23; g(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23) })
    attempt("Overloaded23.apply(xs)")((new Overloaded23: Any).asInstanceOf[FunctionXXL].apply(xs))
