// Scala.js facade syntax, rules N1 to N3: bindings with @JSImport, @JSGlobal and @JSGlobalScope
// nested in objects, native types whose members are plain properties and methods, and JS-object
// classes whose instances are plain objects. Compiled together with tests/interop/scalajs-stub.
// Expectations follow Scala.js: bindings are the imports, an omitted trailing default is not
// passed (`new Date()` is valid, `Date.UTC(2020, 0)` is a number, `scaled()` sees no argument),
// a JS-object instance has one own property per val in declaration order, undefined ones
// included, under the verbatim or @JSName names, and compares by reference.
package native

import scala.scalajs.js
import scala.scalajs.js.annotation.{JSGlobal, JSGlobalScope, JSImport, JSName}

object NodeApi:
  // A def is a call of the import binding; the repeated parameter is spread.
  @js.native @JSImport("node:path", "join")
  def join(parts: String*): String = js.native

  @js.native @JSImport("node:path", JSImport.Namespace)
  object Path extends js.Object:
    def basename(path: String, suffix: String = js.native): String = js.native
    val sep: String = js.native

  @js.native @JSImport("node:os", JSImport.Default)
  object Os extends js.Object:
    def platform(): String = js.native

  // A trailing default that is left out is not passed at all.
  @js.native @JSImport("node:util", "inspect")
  def inspect(value: js.Any, options: js.UndefOr[js.Object] = js.undefined): String = js.native

object Builtins:
  @js.native @JSGlobal("Map")
  class JsMap[K, V] extends js.Object:
    def set(key: K, value: V): JsMap[K, V] = js.native
    def get(key: K): js.UndefOr[V] = js.native
    def has(key: K): Boolean = js.native
    def size: Int = js.native

  @js.native @JSGlobalScope
  object Globals extends js.Object:
    val TEST_FLAG: js.UndefOr[String] = js.native
    val TEST_MISSING: js.UndefOr[String] = js.native
    var TEST_VAR: Int = js.native
    def parseInt(text: String, radix: Int): Int = js.native
    def eval(code: String): js.Any = js.native

  @js.native
  trait Point extends js.Object:
    def x: Double
    var y: Int
    def sum(): Double = js.native
    def scaled(factor: Int = js.native): Double = js.native
    @JSName("data-cy")
    def dataCy: String = js.native

class Opts(
  val a: Int,
  val b: js.UndefOr[String] = js.undefined,
  @JSName("data-cy") val cy: String = "c",
) extends js.Object:
  val `type`: String = "t"
  val extra: js.UndefOr[Int] = js.undefined

class Box[A](val value: A) extends js.Object

// a parameterless def is a getter, so JavaScript reads it as a property
class Counter(val n: Int) extends js.Object:
  def doubled: Int = n * 2
  def add(k: Int): Int = n + k

trait Shape extends js.Object:
  val name: String
  val size: js.UndefOr[Int] = js.undefined

class Square extends Shape:
  val name: String = "square"

def keys(o: js.Object): String = js.Object.keys(o).join(",")

@main def main(): Unit =
  println(NodeApi.join("a", "b", "c.txt"))
  println(NodeApi.join(List("x", "y")*))
  println(NodeApi.Path.basename("/tmp/report.pdf"))
  println(NodeApi.Path.basename("/tmp/report.pdf", ".pdf"))
  println(NodeApi.Path.sep)
  println(js.typeOf(NodeApi.Os.platform()))
  println(NodeApi.inspect(js.Dynamic.literal("a" -> 1)))

  val m = new Builtins.JsMap[String, Int]()
  m.set("one", 1).set("two", 2)
  println(s"${m.size} ${m.has("one")} ${m.get("two")} ${m.get("three").isDefined}")
  val any: Any = m
  any match
    case jm: Builtins.JsMap[?, ?] => println(s"a Map of ${jm.size}")
    case _ => println("something else")
  println((Builtins.Globals: Any).isInstanceOf[Builtins.JsMap[?, ?]])

  // new Date() rather than new Date(undefined), which would be an invalid date
  println(js.typeOf(new js.Date().getTime()))
  println(new js.Date(0).toISOString())
  println(js.Date.UTC(2020, 0) == 1577836800000.0)
  println(js.typeOf(js.Date.now()))
  println(js.JSON.stringify(js.Array(1, 2, 3)))

  _root_.js.set(_root_.js.globalThis, "TEST_FLAG", "on")
  println(Builtins.Globals.TEST_FLAG)
  println(Builtins.Globals.TEST_MISSING.isDefined)
  // a bare assignment, as in Scala.js: the variable has to exist
  _root_.js.set(_root_.js.globalThis, "TEST_VAR", 0)
  Builtins.Globals.TEST_VAR = 41
  Builtins.Globals.TEST_VAR = Builtins.Globals.TEST_VAR + 1
  println(_root_.js.get(_root_.js.globalThis, "TEST_VAR"))
  println(Builtins.Globals.parseInt("ff", 16))

  val p = Builtins.Globals
    .eval("({ x: 1.5, y: 2, \"data-cy\": \"pt\", sum() { return this.x + this.y; }, scaled(f) { return arguments.length === 0 ? -1.5 : this.x * f + 0.5; } })")
    .asInstanceOf[Builtins.Point]
  println(s"${p.x} ${p.y} ${p.sum()} ${p.dataCy}")
  p.y = 10
  println(p.sum())
  println(s"${p.scaled(2)} ${p.scaled()}")

  val o = Opts(1)
  println(keys(o))
  println(js.JSON.stringify(o))
  println(js.JSON.stringify(Opts(2, "x", cy = "y")))
  println(js.JSON.stringify(new Opts(3, b = "z")))
  println(s"${o.a} ${o.b.isDefined} ${o.cy} ${o.`type`} ${o.extra.isDefined}")
  println(Opts(1) == Opts(1))
  println(o == o)
  println(o.isInstanceOf[Opts])
  println((js.Dynamic.literal(): Any).isInstanceOf[Opts])
  println(js.typeOf(o))
  println(js.JSON.stringify(Box(List(1, 2).length)))
  println(Box("s").value.length)

  val counter = Counter(2)
  println(s"${counter.doubled} ${counter.add(1)} ${counter.asInstanceOf[js.Dynamic].doubled}")
  println(js.JSON.stringify(counter))

  val sq = Square()
  println(keys(sq))
  println(js.JSON.stringify(sq))
  println((sq: Shape).name)

  val nullable: String | Null = _root_.js.cast(_root_.js.nullValue)
  val present: String | Null = "text"
  def show(v: String | Null): String = v match
    case s: String => s"string $s"
    case _ => "null"
  println(show(nullable))
  println(show(present))
  println((nullable: Any).isInstanceOf[Null])
  println((present: Any).isInstanceOf[Null])
