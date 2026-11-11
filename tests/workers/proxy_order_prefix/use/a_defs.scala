// The first file: one worker types B, then C, which draw m$proxy1 and m$proxy2 of this unit (dotc prints those).
package defs
object B:
  def arg(): Int = 1
  def text: String = lib.Names.bound(arg())
object C:
  def arg(): Int = 2
  def text = lib.Names.bound(arg())
@main def run(): Unit = println(B.text + "," + C.text)
