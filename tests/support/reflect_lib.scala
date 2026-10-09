//> using platform js
//> using jsVersion 1.21.0
// The jar side of tests/classpath/js/reflect_jar.scala, compiled by scalac for Scala.js: classes
// found by name at run time that the program never names, some inheriting the annotation
// through a class of the jar whose TASTy does not mention it.
package reflectlib

@scala.scalajs.reflect.annotation.EnableReflectiveInstantiation
trait Plugin:
  def name: String

class Hidden(val n: Int) extends Plugin:
  def this() = this(7)
  def name = s"hidden $n"

abstract class Middle extends Plugin

final class Deep(label: String) extends Middle:
  def name = s"deep $label"

class NotPlugin(val x: Int)

class Box(val n: Int):
  class Item(val m: Int) extends Plugin:
    def name = s"item ${n + m}"

// A qualified private constructor counts, a qualified protected one does not.
class QPriv private[reflectlib] () extends Plugin:
  def name = "qpriv"

class QProt protected[reflectlib] () extends Plugin:
  def name = "qprot"

class QSec(x: Int) extends Plugin:
  private[reflectlib] def this() = this(1)
  protected[reflectlib] def this(s: String) = this(2)
  def name = "qsec"

@scala.scalajs.reflect.annotation.EnableReflectiveInstantiation
object Registry:
  def greet = "registry"
  object Inner extends Plugin:
    def name = "inner"

object Loose extends Plugin:
  def name = "loose"
