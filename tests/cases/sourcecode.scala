//> using dep com.lihaoyi::sourcecode:0.4.2
// jars: sourcecode
package demo.app

import sourcecode.{FullName, Line}

def site(tag: String)(using n: sourcecode.Name, f: FullName, l: Line): String =
  s"$tag: ${n.value} | ${f.value} | ${l.value}"

def nameOf(using n: sourcecode.Name): String = n.value
def lineOf(using l: Line): Int = l.value

// Forwarding: the using parameter of the helper wins over a synthesized value.
def forwarded(tag: String)(using n: sourcecode.Name, f: FullName): String =
  "forwarded " + site(tag)
def forwardedTwice(tag: String)(using sourcecode.Name, FullName): String =
  "twice " + forwarded(tag)

def block(body: => String)(using n: sourcecode.Name, l: Line): String =
  s"block ${n.value}:${l.value} " + body

val topVal = site("topVal")
def topDef: String = site("topDef")
val topLambda: Int => String = x => site("topLambda" + x)
lazy val topLazy = site("topLazy")
var topVar = site("topVar")

trait Show[A]:
  def show(a: A): String

given Show[Int] with
  def show(a: Int): String = site("anonymousGiven")

given namedShow: Show[String] with
  def show(a: String): String = site("namedGiven")

case class Tag(value: String)
def mkTag(using n: sourcecode.Name, f: FullName): Tag = Tag(n.value + " | " + f.value)
given Tag = mkTag
given namedTag: Option[Tag] = Some(mkTag)

case class Holder(value: String)
given derivedHolder(using n: sourcecode.Name): Holder = Holder("held by " + n.value)
def holder(using h: Holder): String = h.value

class Widget(val id: Int):
  val field = site("field")
  lazy val lazyField = site("lazyField")
  def method(x: Int): String = site("method")
  def withLocals: String =
    def local(y: Int): String =
      def deeper: String = site("deeper")
      site("local") + "\n" + deeper
    val localVal = site("localVal")
    val localLambda = (z: Int) => site("localLambda")
    val (first, second) = (site("patternVal"), 2)
    local(1) + "\n" + localVal + "\n" + localLambda(1) + "\n" + first
  def inLambda: String = List(1).map(x => site("inLambda")).head
  def inFor: List[String] =
    for x <- List(1, 2) yield site("inFor" + x)
  def inMatch(o: Option[Int]): String = o match
    case Some(v) => site("inMatch")
    case None => site("inMatchNone")
  println(site("classBody"))

class Registered(val id: Int)(using n: sourcecode.Name, l: Line):
  def describe: String = s"Registered($id) as ${n.value} at ${l.value}, forwards $nameOf"

object Outer:
  val inObject = site("inObject")
  println(site("objectBody"))
  object Inner:
    val deep = site("deep")
    def deepDef: String = site("deepDef")
    object Innermost:
      def deepest: String = site("deepest")
  class Nested:
    def member: String = site("nestedClass")
  given Show[Boolean] with
    def show(a: Boolean): String = site("givenInObject")

trait Greeter:
  def greet: String = site("traitDefault")

object EnglishGreeter extends Greeter

enum Color:
  case Red, Green
  def describe: String = site("enumMethod")

extension (x: Int)
  def ext: String = site("extension")

// The shape of the target application's component helpers.
object FC:
  def apply[P](component: P => String)(using name: sourcecode.Name, line: Line): P => String =
    props => s"${name.value}:${line.value} renders " + component(props)

def fc(body: => String)(using name: FullName, line: Line): String =
  s"${name.value}:${line.value} renders " + body

val readMoreText: Int => String = FC[Int]: props =>
  val count = props + 1
  "read more: " + count

object SegmentedSwitch:
  def segmentedSwitch[T](props: T): String = fc:
    val shown = props.toString
    "switch " + shown

def multiLine: String = site(
  "multiLine"
)

def colonBlock: String = block:
  val a = 1
  val b = 2
  s"${a + b}"
  // trailing comment

def nestedCalls: String = site(
  "outer " + site(
    "inner"
  )
)

def chained: Int =
  List(1, 2)
    .map(x => x + lineOf)
    .head

def inExpression: String = "a" + lineOf + "b"

def viaGiven: String = holder

def localGiven: String =
  given sourcecode.Name = sourcecode.Name("explicit")
  site("localGiven") + " / " + nameOf + " / " + forwarded("localGivenForwarded")

def fileName(using f: sourcecode.FileName): String = f.value
def filePath(using f: sourcecode.File): String = f.value

@main def run(): Unit =
  println(topVal)
  println(topDef)
  println(topLambda(1))
  println(topLazy)
  println(topVar)
  println(summon[Show[Int]].show(1))
  println(summon[Show[String]].show(""))
  println(summon[Tag])
  println(summon[Option[Tag]])
  val w = Widget(1)
  println(w.field)
  println(w.lazyField)
  println(w.method(1))
  println(w.withLocals)
  println(w.inLambda)
  println(w.inFor)
  println(w.inMatch(Some(1)))
  println(w.inMatch(None))
  val registered = Registered(7)
  println(registered.describe)
  println(Outer.inObject)
  println(Outer.Inner.deep)
  println(Outer.Inner.deepDef)
  println(Outer.Inner.Innermost.deepest)
  println(Outer.Nested().member)
  println(summon[Show[Boolean]](using Outer.given_Show_Boolean).show(true))
  println(EnglishGreeter.greet)
  println(Color.Red.describe)
  println(1.ext)
  println(readMoreText(2))
  println(SegmentedSwitch.segmentedSwitch(42))
  println(multiLine)
  println(colonBlock)
  println(nestedCalls)
  println(chained)
  println(inExpression)
  println(viaGiven)
  println(localGiven)
  println(forwarded("forwarded"))
  println(forwardedTwice("forwardedTwice"))
  println(nameOf(using sourcecode.Name("passed")))
  println(site("explicit")(using sourcecode.Name("a"), FullName("b.a"), Line(-1)))
  println(sourcecode.Line(3))
  println(sourcecode.Name("x") == sourcecode.Name("x"))
  println(fileName)
  println(filePath.endsWith("/" + fileName))
