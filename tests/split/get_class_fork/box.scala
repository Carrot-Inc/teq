package getclassfork.box

/** The program's getClass calls, over a primitive receiver and a reference one, each the first of the build: dotty's
 *  `Any_getClass`, which Definitions.init forces with the other synthetic core methods (Definitions.scala:309-315,
 *  2209-2221), is the same symbol whichever worker types its first call. */
class Box
object Box:
  def primitive(i: Int): String = i.getClass.getName
  def reference(x: Any): String = x.getClass.getName
  def widened(l: Long): String = l.getClass[Long]().getName
