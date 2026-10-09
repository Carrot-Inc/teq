// Rule N4: an anonymous instance of a JS trait is an object literal. Expectations follow
// Scala.js: the properties are the members the body gives, its vals ahead of its assignments to
// the traits' js.undefined vars, and the js.undefined vals it leaves out are absent. A native
// trait's instance, which Scala.js refuses, keeps the js.undefined vals of its non-native bases,
// ahead of the body's members in source order. Compiled with tests/interop/scalajs-stub.
package jsobjects

import scala.scalajs.js

trait Options[A] extends js.Object:
  val keys: js.Array[String]
  val threshold: js.UndefOr[Double] = js.undefined
  val shouldSort: js.UndefOr[Boolean] = js.undefined

trait Key extends js.Object:
  val name: String
  val weight: js.UndefOr[Double] = js.undefined

object Key:
  def apply(nameArg: String): Key =
    new Key:
      val name: String = nameArg

@js.native
trait ListenerOptions extends js.Object:
  var capture: js.UndefOr[Boolean] = js.native
  var once: js.UndefOr[Boolean] = js.native
  var passive: js.UndefOr[Boolean] = js.native

@js.native
trait RequestInit extends js.Object:
  var method: js.UndefOr[String] = js.native
  var body: js.UndefOr[String] = js.native

@js.native
trait Header extends js.Object:
  var name: String = js.native
  val value: String = js.native

trait Optional extends js.Object:
  var tag: js.UndefOr[String] = js.undefined

@js.native
trait Tagged extends Optional:
  val label: String = js.native

trait Named extends js.Object:
  val name: String

trait Sized extends js.Object:
  val size: Int

def stringify(o: js.Any): String = js.JSON.stringify(o)
def keysOf(o: js.Object): String = js.Object.keys(o).join(",")
def count(o: Options[Int]): Int = o.keys.length

@main def main(): Unit =
  val limit = 0.4
  val o = new Options[Int] {
    val keys = js.Array("title", "body")
    override val threshold = limit
  }
  println(stringify(o))
  println(keysOf(o))
  println(s"${count(o)} ${o.threshold.getOrElse(1.0)} ${o.shouldSort.isDefined}")

  val typed: Options[String] = new Options:
    val keys = js.Array("x")
    override val shouldSort: js.UndefOr[Boolean] = true
  println(stringify(typed))

  val k = Key("k")
  println(keysOf(k))
  println(stringify(k))
  println(k.name)

  val listener = new ListenerOptions { once = true }
  println(keysOf(listener))
  println(stringify(listener))
  println(js.isUndefined(listener.capture))

  val request = new RequestInit {}
  println(stringify(request))
  request.method = "POST"
  println(stringify(request))
  val get = new RequestInit:
    method = "GET"
    body = "payload"
  println(stringify(get))
  println(get.body)
  val header = new Header { name = "accept"; val value = "text/html" }
  println(keysOf(header))
  println(keysOf(new Tagged { tag = "t"; val label = "l" }))
  println(keysOf(new Tagged { val label = "m" }))

  val both = new Named with Sized {
    val name = "n"
    val size = 2
  }
  println(stringify(both))
  println(s"${(both: Named).name} ${(both: Sized).size}")

  println(stringify(new js.Object))
  println(keysOf(new js.Object))

  // a @JSBracketAccess trait built by an @js intrinsic named apply in a nested object
  val dict = js.Dictionary("a" -> 1, "b" -> 2)
  dict("c") = 3
  println(s"${stringify(dict)} ${dict("a") + dict("c")}")

  // js.UndefOr[A] is A | Unit; an extension on A | Unit peels A off
  val maybe: js.UndefOr[Key] = Key("peeled")
  val absent: js.UndefOr[Key] = js.undefined
  println(maybe.toOption.map(_.name))
  println(absent.toOption.map(_.name))
  println(maybe.map(_.name.length))
