//> using platform js
// A primitive's box and `()` tested against the interfaces and classes the JVM's boxes extend
// (`Serializable`, `Comparable`, `CharSequence`, `Number`), alone, with `AnyRef` and with each other, by
// `isInstanceOf`, by a pattern and by `Class.isInstance`: on JavaScript as Scala.js answers (the box is the
// primitive; `()`'s class extends `Object` alone, so `()` is no `Serializable` there), on the JVM and in the
// interpreter as scalac's JVM does (`tests/jvm-expected`; `BoxedUnit` is `Serializable`). JavaScript answered
// `()` as a `Serializable`. A `Char`, a string on JavaScript, is left out (docs/COMPATIBILITY.md).
object BoxInterfaces {
  def t_ser(x: Any): Boolean = x.isInstanceOf[java.io.Serializable]
  def p_ser(x: Any): Boolean = x match { case _: (java.io.Serializable) => true; case _ => false }
  def t_cmp(x: Any): Boolean = x.isInstanceOf[Comparable[?]]
  def p_cmp(x: Any): Boolean = x match { case _: (Comparable[?]) => true; case _ => false }
  def t_cs(x: Any): Boolean = x.isInstanceOf[CharSequence]
  def p_cs(x: Any): Boolean = x match { case _: (CharSequence) => true; case _ => false }
  def t_num(x: Any): Boolean = x.isInstanceOf[Number]
  def p_num(x: Any): Boolean = x match { case _: (Number) => true; case _ => false }
  def t_rSer(x: Any): Boolean = x.isInstanceOf[AnyRef & java.io.Serializable]
  def p_rSer(x: Any): Boolean = x match { case _: (AnyRef & java.io.Serializable) => true; case _ => false }
  def t_rCmp(x: Any): Boolean = x.isInstanceOf[AnyRef & Comparable[?]]
  def p_rCmp(x: Any): Boolean = x match { case _: (AnyRef & Comparable[?]) => true; case _ => false }
  def t_rCs(x: Any): Boolean = x.isInstanceOf[AnyRef & CharSequence]
  def p_rCs(x: Any): Boolean = x match { case _: (AnyRef & CharSequence) => true; case _ => false }
  def t_rNum(x: Any): Boolean = x.isInstanceOf[AnyRef & Number]
  def p_rNum(x: Any): Boolean = x match { case _: (AnyRef & Number) => true; case _ => false }
  def t_serCmp(x: Any): Boolean = x.isInstanceOf[java.io.Serializable & Comparable[?]]
  def p_serCmp(x: Any): Boolean = x match { case _: (java.io.Serializable & Comparable[?]) => true; case _ => false }
  def t_csCmp(x: Any): Boolean = x.isInstanceOf[CharSequence & Comparable[?]]
  def p_csCmp(x: Any): Boolean = x match { case _: (CharSequence & Comparable[?]) => true; case _ => false }
  def t_numCmp(x: Any): Boolean = x.isInstanceOf[Number & Comparable[?]]
  def p_numCmp(x: Any): Boolean = x match { case _: (Number & Comparable[?]) => true; case _ => false }
  def t_serCs(x: Any): Boolean = x.isInstanceOf[java.io.Serializable & CharSequence]
  def p_serCs(x: Any): Boolean = x match { case _: (java.io.Serializable & CharSequence) => true; case _ => false }
  def t_numSer(x: Any): Boolean = x.isInstanceOf[Number & java.io.Serializable]
  def p_numSer(x: Any): Boolean = x match { case _: (Number & java.io.Serializable) => true; case _ => false }
  def t_rSerCmp(x: Any): Boolean = x.isInstanceOf[AnyRef & java.io.Serializable & Comparable[?]]
  def p_rSerCmp(x: Any): Boolean = x match { case _: (AnyRef & java.io.Serializable & Comparable[?]) => true; case _ => false }
  def bits(x: Any): String = Seq(t_ser(x), p_ser(x), t_cmp(x), p_cmp(x), t_cs(x), p_cs(x), t_num(x), p_num(x), t_rSer(x), p_rSer(x), t_rCmp(x), p_rCmp(x), t_rCs(x), p_rCs(x), t_rNum(x), p_rNum(x), t_serCmp(x), p_serCmp(x), t_csCmp(x), p_csCmp(x), t_numCmp(x), p_numCmp(x), t_serCs(x), p_serCs(x), t_numSer(x), p_numSer(x), t_rSerCmp(x), p_rSerCmp(x)).map(b => if (b) '1' else '0').mkString
  def classes(x: Any): String = Seq(classOf[java.io.Serializable].isInstance(x), classOf[Comparable[?]].isInstance(x), classOf[CharSequence].isInstance(x), classOf[Number].isInstance(x)).map(b => if (b) '1' else '0').mkString
  def main(args: Array[String]): Unit = {
    println("tests: ser cmp cs num rSer rCmp rCs rNum serCmp csCmp numCmp serCs numSer rSerCmp (isInstanceOf, then a pattern); classes: Ser Cmp Cs Num")
    println("boolean " + bits(true) + " " + classes(true))
    println("byte    " + bits(1.toByte) + " " + classes(1.toByte))
    println("short   " + bits(2.toShort) + " " + classes(2.toShort))
    println("int     " + bits(3) + " " + classes(3))
    println("long    " + bits(4L) + " " + classes(4L))
    println("float   " + bits(1.5f) + " " + classes(1.5f))
    println("double  " + bits(2.5) + " " + classes(2.5))
    println("unit    " + bits(()) + " " + classes(()))
    println("string  " + bits("s") + " " + classes("s"))
    println("null    " + bits(null) + " " + classes(null))
  }
}
