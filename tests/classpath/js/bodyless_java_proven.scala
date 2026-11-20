// jars: bodylessjava-lib
// targets: interp
// Creations whose Java levels the interpreter proves do nothing it leaves undone (src/interp/bodyless.rs): a
// constructor storing its field's default under Object (jlib.Counter), the empty one of an overloaded pair
// (jlib.Span()), a Java interface whose default is not called (jlib.Quiet), and the JDK's AbstractList, whose
// `modCount = 0` follows the construction of the std's AbstractCollection, which does nothing.
package proven

class Tick extends jlib.Counter
class Width extends jlib.Span
class Calm extends jlib.Quiet
class Items extends java.util.AbstractList[Int]:
  def get(i: Int): Int = i
  def size(): Int = 2
  def changes: Int = modCount

@main def run(): Unit =
  println(new Tick().count)
  println(new Width().n)
  println(new Calm().isInstanceOf[jlib.Quiet])
  println(new Items().changes)
