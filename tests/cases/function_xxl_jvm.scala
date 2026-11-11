//> using platform jvm
// std: scala-library
// The class functions past 22 parameters erase to on the JVM, scala-library's scala.runtime.FunctionXXL
// (`Definitions.functionTypeErasure`): a lambda of 23 or 64 parameters and an instance of a class extending
// Function23 are FunctionXXLs, called with their arguments in one array (FunctionXXLForwarders' `apply(Object[])` in
// the class), and the class takes FunctionXXL's `toString` (`Mixin`'s forwarder of the erased parent's member).
package function_xxl_jvm

import scala.runtime.FunctionXXL

class Sum23 extends Function23[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int]:
  def apply(a1: Int, a2: Int, a3: Int, a4: Int, a5: Int, a6: Int, a7: Int, a8: Int, a9: Int, a10: Int, a11: Int, a12: Int, a13: Int, a14: Int, a15: Int, a16: Int, a17: Int, a18: Int, a19: Int, a20: Int, a21: Int, a22: Int, a23: Int): Int = a1 + a2 + a3 + a4 + a5 + a6 + a7 + a8 + a9 + a10 + a11 + a12 + a13 + a14 + a15 + a16 + a17 + a18 + a19 + a20 + a21 + a22 + a23

@main def main(): Unit =
  val f: (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int) => Int = (a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23) => a1 * 1 + a2 * 2 + a3 * 3 + a4 * 4 + a5 * 5 + a6 * 6 + a7 * 7 + a8 * 8 + a9 * 9 + a10 * 10 + a11 * 11 + a12 * 12 + a13 * 13 + a14 * 14 + a15 * 15 + a16 * 16 + a17 * 17 + a18 * 18 + a19 * 19 + a20 * 20 + a21 * 21 + a22 * 22 + a23 * 23
  val g: (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int) => Int = (a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23, a24, a25, a26, a27, a28, a29, a30, a31, a32, a33, a34, a35, a36, a37, a38, a39, a40, a41, a42, a43, a44, a45, a46, a47, a48, a49, a50, a51, a52, a53, a54, a55, a56, a57, a58, a59, a60, a61, a62, a63, a64) => a1 + a2 + a3 + a4 + a5 + a6 + a7 + a8 + a9 + a10 + a11 + a12 + a13 + a14 + a15 + a16 + a17 + a18 + a19 + a20 + a21 + a22 + a23 + a24 + a25 + a26 + a27 + a28 + a29 + a30 + a31 + a32 + a33 + a34 + a35 + a36 + a37 + a38 + a39 + a40 + a41 + a42 + a43 + a44 + a45 + a46 + a47 + a48 + a49 + a50 + a51 + a52 + a53 + a54 + a55 + a56 + a57 + a58 + a59 + a60 + a61 + a62 + a63 + a64
  val s = new Sum23
  for (name, x) <- List[(String, Any)](("f", f), ("g", g), ("s", s)) do
    println(name + " " + x.isInstanceOf[FunctionXXL] + " " + x.isInstanceOf[Function23[?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?]])
  val xs: Array[Object] = Array.tabulate[Object](23)(i => Integer.valueOf(i + 1))
  println(f.asInstanceOf[FunctionXXL].apply(IArray.unsafeFromArray(xs)))
  println(s.asInstanceOf[FunctionXXL].apply(IArray.unsafeFromArray(xs)))
  println(g.asInstanceOf[FunctionXXL].apply(IArray.unsafeFromArray(Array.tabulate[Object](64)(i => Integer.valueOf(i + 1)))))
  println(s.toString)
  println((s: Any).asInstanceOf[(Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int) => Int](1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))
