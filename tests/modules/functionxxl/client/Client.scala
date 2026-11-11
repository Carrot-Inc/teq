package fxc

import fxa.*
import scala.runtime.FunctionXXL

// scalac's caller of the API, compiled over a's products, teq's or scalac's, and run over teq's class files: the
// calls name FunctionXXL, scalac's erasure of every function class past 22 parameters, and the classes a defines
// are FunctionXXLs whose `apply(Object[])` and `toString` scalac's own would have.
object Client:
  def attempt(name: String)(f: => Any): Unit =
    try println(name + ": " + f)
    catch case e: LinkageError => println(name + ": " + e.getClass.getSimpleName)

  def main(args: Array[String]): Unit =
    val c = new Callee
    attempt("id23(null)")(c.id23(null))
    attempt("id023(null)")(c.id023(null))
    attempt("mk(1..23)")(c.mk(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))
    attempt("mk64(1..64)")(c.mk64(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64))
    attempt("id023(lambda)")(c.id023((a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23) => a1 + a2 + a3 + a4 + a5 + a6 + a7 + a8 + a9 + a10 + a11 + a12 + a13 + a14 + a15 + a16 + a17 + a18 + a19 + a20 + a21 + a22 + a23)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))
    attempt("Last23 via id23")(c.id23(Last23)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))
    attempt("Sum23C via Function23")({ val g: Function23[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] = new Sum23C; g(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23) })
    attempt("FunctionErasure.id(null)")(new FunctionErasure().id(null))
    val xs = IArray.unsafeFromArray(Array.tabulate[Object](23)(i => Integer.valueOf(i + 1)))
    attempt("mk is a FunctionXXL")(c.mk.isInstanceOf[FunctionXXL])
    attempt("mk.apply(xs)")(c.mk.asInstanceOf[FunctionXXL].apply(xs))
    attempt("Sum23C is a FunctionXXL")((new Sum23C: Any).isInstanceOf[FunctionXXL])
    attempt("Sum23C.apply(xs)")((new Sum23C: Any).asInstanceOf[FunctionXXL].apply(xs))
    attempt("Sum23C.toString")(new Sum23C().toString)
    attempt("Last23.apply(xs)")((Last23: Any).asInstanceOf[FunctionXXL].apply(xs))
