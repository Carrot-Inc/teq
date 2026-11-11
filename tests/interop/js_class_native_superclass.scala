// A JS class that extends a native JS class is a JavaScript subclass of the class the facade's load spec names, and
// its constructor's super call passes the parent's arguments (Scala.js's JSCodeGen: the superclass is the native
// class, constructed with the arguments as `new` passes them): its instances inherit the native methods and getters
// and pass the native class's type test; an argument runs once a construction. The expectation is scalac 3.8.4's
// under Scala.js 1.22.0 (`scala-cli run --js --js-version 1.22.0 --scala 3.8.4`).
import scala.scalajs.js
import scala.scalajs.js.annotation.*

@js.native
@JSGlobal("Date")
class Moment(value: Double = js.native) extends js.Object:
  def this(year: Int, month: Int) = this(0.0)
  def getTime(): Double = js.native
  def getFullYear(): Int = js.native
  def getMonth(): Int = js.native
  def toISOString(): String = js.native

@js.native
@JSGlobal("RegExp")
class Pattern(pattern: String, flags: String = js.native) extends js.Object:
  def source: String = js.native

@js.native
@JSGlobal("Array")
class NativeArray(items: Int*) extends js.Object:
  val length: Int = js.native

class Three extends NativeArray(1, 2, 3)

var log = List.empty[String]
def note(s: String): Double =
  log = log :+ s
  0.0

class Epoch(t: Double) extends Moment(t):
  val twice: Double = getTime() * 2

class Now extends Moment:
  def recent: Boolean = getTime() > 1.0e12

class Counted extends Moment(note("argument")):
  def time: Double = getTime()

class June extends Moment(2020, 5):
  def month: Int = getMonth()

class Later(t: Double) extends Epoch(t + 1000):
  def seconds: Double = getTime() / 1000

object Origin extends Moment(86400000.0):
  def day: Double = getTime() / 86400000.0

class Abc extends Pattern("abc"):
  def source_=(v: String): Unit = ()

def local(): Double =
  class Local extends Moment(5.0):
    def t: Double = getTime()
  new Local().t

@main def run(): Unit =
  val e = new Epoch(0)
  println(e.getTime())
  println(e.twice)
  println(e.toISOString())
  println((e: Any).isInstanceOf[Moment])
  println(new Now().recent)
  val c1 = new Counted
  val c2 = new Counted
  println(log)
  println(c1.time + c2.time)
  val j = new June
  println(j.getFullYear())
  println(j.month)
  val l = new Later(0)
  println(l.seconds)
  println((l: Any).isInstanceOf[Epoch])
  println(Origin.day)
  println(local())
  println(new Abc().source)
  println(new Three().length)
