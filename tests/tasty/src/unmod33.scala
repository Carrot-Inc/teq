package fix.unmod

import scala.scalajs.js

// A body compiled with Scala 3.3, which pickles `p.writable = true` on a JavaScript trait's var
// as a call of its setter, `p.writable_=(true)`.
object UmSetter33:
  @js.native
  private trait Prop extends js.Object with js.PropertyDescriptor:
    val key: String = js.native
  private def Prop(key: String, v: Any): Prop = js.Dynamic.literal(key = key, value = v.asInstanceOf[js.Any]).asInstanceOf[Prop]
  def define(target: js.Object, key: String, v: Any): Unit =
    val p = Prop(key, v)
    p.configurable = true
    p.writable = true
    js.Object.defineProperty(target, p.key, p)

// A case class companion calling its synthesized `apply` unqualified, which the pickle refers to
// by the definition's address.
final case class UmArg[A](render: A => String)

object UmArg:
  def unit[F[_]]: UmArg[F[Unit]] = apply(_ => "unit")
  def list[A](show: A => String): UmArg[List[A]] = apply(_.map(show).mkString("[", ",", "]"))

// `new` of an alias that fixes some of its class's type parameters, which the pickle spells as
// the alias applied to the class's own type arguments (scalajs-react's `new Lifecycle.RenderScope`).
class UmScope[F[_], P](val p: P):
  def show: String = s"scope($p)"

object UmAliases:
  type Scope[P] = UmScope[Option, P]
  def make[P](p: P): Scope[P] = new Scope[P](p)

// A method named like a setter with no getter beside it, called by its name.
class UmSetterOnly:
  var seen = 0
  def x_=(v: Int): Unit = seen = v

object UmSetterUse:
  def use(c: UmSetterOnly): Int =
    c.x_=(7)
    c.seen
